struct Rectangle {
    width: u32,
    height: u32,
}

impl Rectangle {
    fn area(self) -> u32 {
        self.width * self.height
    }
}

fn main() {
    let rect1 = Rectangle {
        width: 30,
        height: 50,
    };

    let rect2 = Rectangle {
        width: 30,
        height: 50,
    };
   // println!("{}",rect1.area());

     let v = vec![rect1,rect2];
    let a:Vec<i32> = v.into_iter().map(|item| item.area()).collect();
    // let a = v.into_iter().map(|ele| -> ele.area()).collect();



}