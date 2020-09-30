

pub enum Error {
    First,
    Second,
}

impl Error {
    fn error(self) {
        match self {
            self::Fist => println!("hellp"),
            self::Second => println!("hello2"),
        }
    }
}