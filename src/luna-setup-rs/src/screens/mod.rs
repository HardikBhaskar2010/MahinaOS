pub mod welcome;
pub mod disk_select;
pub mod user_info;
pub mod progress;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WizardState {
    Welcome,
    DiskSelect,
    UserInfo,
    Progress,
    Done,
}

pub trait Screen {
    fn render(&mut self, pixels: &mut [u8], stride: u32, w: u32, h: u32);
    fn on_click(&mut self, x: i32, y: i32) -> Option<WizardState>;
    fn on_key(&mut self, key: u32, pressed: bool);
}
