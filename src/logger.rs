use chrono::offset::Local;

pub fn log<S: AsRef<str>>(s: S) {
    let dt = Local::now();
    let sdt = dt.format("%Y-%m-%d %H:%M:%S:%.3f");
    println!("{}: {}", sdt, s.as_ref());
}
