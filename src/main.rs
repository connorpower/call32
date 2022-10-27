use ::windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageA, GetMessageA, TranslateMessage, MSG,
};
use ::directx::{geom::Dimension2D, win32::{*, window::Window}};

pub fn main() {
    match run() {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{e}");
        }
    }
}

fn run() -> Result<()> {
    let on_paint = || {
        println!("on paint!");
    };

    let _main_window = Window::new(Dimension2D{ width: 800, height: 600 }, "My Window!", on_paint)?;

    // TODO: shift to a tokio loop
    let mut msg = MSG::default();
    while unsafe { GetMessageA(&mut msg, None, 0, 0) }.as_bool() {
        unsafe { TranslateMessage(&msg) };
        unsafe { DispatchMessageA(&msg) };
    }

    Ok(())
}
