use na::{self, DVectorSlice, DVectorSliceMut, Real};

use math::{Force, Inertia, Isometry, Point, Rotation, Translation, Vector, Velocity, SPATIAL_DIM};
use object::{ActivationStatus, BodyHandle, BodyStatus};
use solver::IntegrationParameters;

#[cfg(feature = "dim3")]
use math::AngularVector;
#[cfg(feature = "dim3")]
use utils::GeneralizedCross;

pub struct RigidBody<N: Real> {
    handle: BodyHandle,
    local_to_world: Isometry<N>,
    velocity: Velocity<N>,
    local_inertia: Inertia<N>,
    inertia: Inertia<N>,
    local_com: Point<N>,
    com: Point<N>,
    augmented_mass: Inertia<N>,
    inv_augmented_mass: Inertia<N>,
    external_forces: Force<N>,
    acceleration: Velocity<N>,
    status: BodyStatus,
    activation: ActivationStatus<N>,
    companion_id: usize,
}

impl<N: Real> RigidBody<N> {
    pub fn new(
        handle: BodyHandle,
        position: Isometry<N>,
        local_inertia: Inertia<N>,
        local_com: Point<N>,
    ) -> Self {
        let inertia = local_inertia.transformed(&position);
        let com = position * local_com;

        RigidBody {
            handle,
            local_to_world: position,
            velocity: Velocity::zero(),
            local_inertia,
            inertia,
            local_com,
            com,
            augmented_mass: inertia,
            inv_augmented_mass: inertia.inverse(),
            external_forces: Force::zero(),
            acceleration: Velocity::zero(),
            status: BodyStatus::Dynamic,
            activation: ActivationStatus::new_active(),
            companion_id: 0,
        }
    }

    #[inline]
    pub fn activation_status(&self) -> &ActivationStatus<N> {
        &self.activation
    }

    #[inline]
    pub fn activation_status_mut(&mut self) -> &mut ActivationStatus<N> {
        &mut self.activation
    }

    #[inline]
    pub fn activate(&mut self) {
        if let Some(threshold) = self.activation.deactivation_threshold() {
            self.activate_with_energy(threshold * na::convert(2.0));
        }
    }

    #[inline]
    pub fn activate_with_energy(&mut self, energy: N) {
        self.activation.set_energy(energy)
    }

    #[inline]
    pub fn deactivate(&mut self) {
        self.activation.set_energy(N::zero());
        self.velocity = Velocity::zero();
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        match self.status {
            BodyStatus::Dynamic => self.activation.is_active(),
            BodyStatus::Kinematic => true,
            BodyStatus::Static => false,
            BodyStatus::Disabled => false,
        }
    }

    #[inline]
    pub fn status(&self) -> BodyStatus {
        self.status
    }

    #[inline]
    pub fn set_status(&mut self, status: BodyStatus) {
        self.status = status
    }

    #[inline]
    pub fn companion_id(&self) -> usize {
        self.companion_id
    }

    #[inline]
    pub fn set_companion_id(&mut self, id: usize) {
        self.companion_id = id
    }

    #[inline]
    pub fn is_dynamic(&self) -> bool {
        self.status == BodyStatus::Dynamic
    }

    #[inline]
    pub fn is_static(&self) -> bool {
        self.status == BodyStatus::Static
    }

    #[inline]
    pub fn is_kinematic(&self) -> bool {
        self.status == BodyStatus::Kinematic
    }

    #[inline]
    pub fn center_of_mass(&self) -> Point<N> {
        self.com
    }

    /// The velocity of this rigid body.
    #[inline]
    pub fn velocity(&self) -> &Velocity<N> {
        &self.velocity
    }

    /// Sets the velocity of this rigid body.
    #[inline]
    pub fn set_velocity(&mut self, vel: Velocity<N>) {
        self.velocity = vel
    }

    /// Sets the linear velocity of this rigid body.
    #[inline]
    pub fn set_linear_velocity(&mut self, vel: Vector<N>) {
        self.velocity.linear = vel
    }

    #[cfg(feature = "dim2")]
    /// Sets the angular velocity of this rigid body.
    #[inline]
    pub fn set_angular_velocity(&mut self, vel: N) {
        self.velocity.angular = vel
    }

    #[cfg(feature = "dim3")]
    /// Sets the angular velocity of this rigid body.
    #[inline]
    pub fn set_angular_velocity(&mut self, vel: AngularVector<N>) {
        self.velocity.angular = vel
    }

    pub fn clear_dynamics(&mut self) {
        self.augmented_mass = Inertia::zero();
        self.acceleration = Velocity::zero();
        self.external_forces = Force::zero();
    }

    pub fn update_dynamics(&mut self, gravity: &Vector<N>, params: &IntegrationParameters<N>) {
        match self.status {
            BodyStatus::Dynamic => {
                // The inverse inertia matrix is constant in 2D.
                #[cfg(feature = "dim3")]
                {
                    self.inertia = self.local_inertia.transformed(&self.local_to_world);
                    self.augmented_mass += self.inertia;

                    let i = &self.inertia.angular;
                    let w = &self.velocity.angular;
                    let iw = i * w;
                    let w_dt = w * params.dt;
                    let w_dt_cross = w_dt.gcross_matrix();
                    let iw_dt_cross = (iw * params.dt).gcross_matrix();
                    self.augmented_mass.angular += w_dt_cross * i - iw_dt_cross;

                    // NOTE: if we did not have the gyroscopic forces, we would not have to invert the inertia
                    // matrix at each time-step => add a flag to disable gyroscopic forces?
                    self.inv_augmented_mass = self.augmented_mass.inverse();

                    /*
                     * Compute acceleration due to gyroscopic forces.
                     */
                    let gyroscopic = -w.cross(&iw);
                    self.acceleration.angular += self.inv_augmented_mass.angular * gyroscopic;
                }

                self.acceleration.linear += *gravity;
                self.acceleration += self.inv_augmented_mass * self.external_forces
            }
            _ => {}
        }
    }

    #[inline]
    pub fn local_inertia(&self) -> &Inertia<N> {
        &self.local_inertia
    }

    #[inline]
    pub fn inertia(&self) -> &Inertia<N> {
        &self.inertia
    }

    #[inline]
    pub fn augmented_mass(&self) -> &Inertia<N> {
        &self.augmented_mass
    }

    #[inline]
    pub fn handle(&self) -> BodyHandle {
        self.handle
    }

    #[inline]
    pub fn ndofs(&self) -> usize {
        SPATIAL_DIM
    }

    #[inline]
    pub fn generalized_velocity(&self) -> DVectorSlice<N> {
        DVectorSlice::from_slice(self.velocity.as_slice(), SPATIAL_DIM)
    }

    #[inline]
    pub fn generalized_velocity_mut(&mut self) -> DVectorSliceMut<N> {
        DVectorSliceMut::from_slice(self.velocity.as_mut_slice(), SPATIAL_DIM)
    }

    #[inline]
    pub fn generalized_acceleration(&self) -> DVectorSlice<N> {
        DVectorSlice::from_slice(self.acceleration.as_slice(), SPATIAL_DIM)
    }

    #[inline]
    pub fn integrate(&mut self, params: &IntegrationParameters<N>) {
        let disp = self.velocity * params.dt;
        self.apply_displacement(&disp);
    }

    #[inline]
    pub fn apply_displacement(&mut self, displacement: &Velocity<N>) {
        let rotation = Rotation::new(displacement.angular);
        let translation = Translation::from_vector(displacement.linear);
        let shift = Translation::from_vector(self.com.coords);
        let disp = translation * shift * rotation * shift.inverse();
        self.local_to_world = disp * self.local_to_world;
        self.com = self.local_to_world * self.local_com;
    }

    #[inline]
    pub fn apply_force(&mut self, force: &Force<N>) {
        self.external_forces.linear += force.linear;
        self.external_forces.angular += force.angular;
    }

    #[inline]
    pub fn position(&self) -> Isometry<N> {
        self.local_to_world
    }

    #[inline]
    pub fn body_jacobian_mul_force(&self, force: &Force<N>, out: &mut [N]) {
        out[..SPATIAL_DIM].copy_from_slice(force.as_slice());
    }

    #[inline]
    pub fn inv_mass_mul_generalized_forces(&self, out: &mut [N]) {
        let force = Force::from_slice(out);
        self.inv_mass_mul_force(&force, out)
    }

    #[inline]
    pub fn inv_mass_mul_force(&self, force: &Force<N>, out: &mut [N]) {
        let acc = self.inv_augmented_mass * *force;
        out[..SPATIAL_DIM].copy_from_slice(acc.as_slice());
    }
}
