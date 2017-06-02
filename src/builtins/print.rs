use parser::Expression;


/// Print the given expressions to standard output. Multiple arguments are separated with a space.
pub fn main(args: &[Expression]) {
    let mut first = true;

    for arg in args {
        if first {
            print!("{}", arg);
            first = false;
        } else {
            print!(" {}", arg);
        }
    }

    println!();
}
