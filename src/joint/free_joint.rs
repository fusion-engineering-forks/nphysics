use na::Real;

use joint::Joint;
use solver::IntegrationParameters;
use math::{Isometry, JacobianSliceMut, Vector, Velocity, SPATIAL_DIM};

#[derive(Copy, Clone, Debug)]
pub struct FreeJoint<N: Real> {
    position: Isometry<N>,
}

impl<N: Real> FreeJoint<N> {
    pub fn new(position: Isometry<N>) -> Self {
        FreeJoint { position }
    }

    fn apply_displacement(&mut self, disp: &Velocity<N>) {
        let disp = Isometry::new(disp.linear, disp.angular);
        self.position = Isometry::from_parts(
            disp.translation * self.position.translation,
            disp.rotation * self.position.rotation,
        )
    }
}

impl<N: Real> Joint<N> for FreeJoint<N> {
    fn ndofs(&self) -> usize {
        SPATIAL_DIM
    }

    fn body_to_parent(&self, _: &Vector<N>, _: &Vector<N>) -> Isometry<N> {
        self.position
    }

    fn update_jacobians(&mut self, _: &Vector<N>, _: &[N]) {}

    fn jacobian(&self, _: &Isometry<N>, out: &mut JacobianSliceMut<N>) {
        out.fill_diagonal(N::one());
    }

    fn jacobian_dot(&self, _: &Isometry<N>, _: &mut JacobianSliceMut<N>) {}

    fn jacobian_dot_veldiff_mul_coordinates(
        &self,
        _: &Isometry<N>,
        _: &[N],
        _: &mut JacobianSliceMut<N>,
    ) {
    }

    fn integrate(&mut self, params: &IntegrationParameters<N>, vels: &[N]) {
        let disp = Velocity::from_slice(vels) * params.dt;
        self.apply_displacement(&disp);
    }

    fn apply_displacement(&mut self, disp: &[N]) {
        let disp = Velocity::from_slice(disp);
        self.apply_displacement(&disp);
    }

    fn jacobian_mul_coordinates(&self, vels: &[N]) -> Velocity<N> {
        Velocity::from_slice(vels)
    }

    fn jacobian_dot_mul_coordinates(&self, _: &[N]) -> Velocity<N> {
        // The jacobian derivative is zero.
        Velocity::zero()
    }
}