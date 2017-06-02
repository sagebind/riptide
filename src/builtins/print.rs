use parser::Expression;


pub fn main(args: &[Expression]) {
    for arg in args {
        println!("{}", arg);
    }
}
