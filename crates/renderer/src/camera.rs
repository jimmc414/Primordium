use glam::{Mat4, Vec3};

pub struct Camera {
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub target: Vec3,
    pub fov_y: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    pub clip_axis: Option<u32>,
    pub clip_position: f32,
}

impl Camera {
    pub fn new(grid_size: u32) -> Self {
        let half = grid_size as f32 * 0.5;
        Self {
            distance: grid_size as f32 * 1.8,
            yaw: 0.4,
            pitch: 0.5,
            target: Vec3::new(half, half, half),
            fov_y: std::f32::consts::FRAC_PI_4,
            aspect: 1.0,
            near: 0.1,
            far: grid_size as f32 * 5.0,
            clip_axis: None,
            clip_position: 0.5,
        }
    }

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw += dx * 0.005;
        self.pitch = (self.pitch + dy * 0.005).clamp(-1.5, 1.5);
    }

    pub fn zoom(&mut self, delta: f32) {
        self.distance = (self.distance * (1.0 - delta * 0.001)).max(1.0);
    }

    pub fn pan(&mut self, dx: f32, dy: f32) {
        let eye = self.eye_position();
        let forward = (self.target - eye).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward).normalize();
        let scale = self.distance * 0.002;
        self.target += right * (-dx * scale) + up * (dy * scale);
    }

    pub fn cycle_clip_axis(&mut self) {
        self.clip_axis = match self.clip_axis {
            None => Some(0),    // X
            Some(0) => Some(1), // Y
            Some(1) => Some(2), // Z
            Some(_) => None,    // Off
        };
    }

    pub fn adjust_clip_position(&mut self, delta: f32) {
        self.clip_position = (self.clip_position + delta).clamp(0.0, 1.0);
    }

    pub fn eye_position(&self) -> Vec3 {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();
        self.target + Vec3::new(x, y, z)
    }

    pub fn view_projection(&self) -> Mat4 {
        let eye = self.eye_position();
        let view = Mat4::look_at_rh(eye, self.target, Vec3::Y);
        let proj = Mat4::perspective_rh(self.fov_y, self.aspect, self.near, self.far);
        proj * view
    }

    pub fn view_projection_inverse(&self) -> Mat4 {
        self.view_projection().inverse()
    }

    /// Serialize camera uniform data for GPU.
    /// Layout: inv_view_proj (16 floats), camera_pos (3 floats + pad),
    ///         grid_size (f32), clip_axis (u32 as f32), clip_position (f32), padding (f32)
    pub fn to_uniform_bytes(&self, grid_size: u32) -> Vec<u8> {
        let inv_vp = self.view_projection_inverse();
        let eye = self.eye_position();
        let clip_axis_val: f32 = match self.clip_axis {
            Some(a) => a as f32,
            None => -1.0,
        };

        let mut bytes = Vec::with_capacity(96);
        // mat4: 16 floats
        for col in 0..4 {
            let c = inv_vp.col(col);
            bytes.extend_from_slice(&c.x.to_le_bytes());
            bytes.extend_from_slice(&c.y.to_le_bytes());
            bytes.extend_from_slice(&c.z.to_le_bytes());
            bytes.extend_from_slice(&c.w.to_le_bytes());
        }
        // camera_pos: vec3 + pad
        bytes.extend_from_slice(&eye.x.to_le_bytes());
        bytes.extend_from_slice(&eye.y.to_le_bytes());
        bytes.extend_from_slice(&eye.z.to_le_bytes());
        bytes.extend_from_slice(&0.0f32.to_le_bytes()); // padding
        // grid_size, clip_axis, clip_position, padding
        bytes.extend_from_slice(&(grid_size as f32).to_le_bytes());
        bytes.extend_from_slice(&clip_axis_val.to_le_bytes());
        bytes.extend_from_slice(&self.clip_position.to_le_bytes());
        bytes.extend_from_slice(&0.0f32.to_le_bytes()); // padding
        bytes
    }
}
