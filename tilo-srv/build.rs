fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/tilo.ico");
    res.compile().unwrap();
}
