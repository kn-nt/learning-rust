fn main() {

    let alpha = vec![1, 2, 3, 4, 5];

    for i in 0..alpha.len() {
        println!("{}", i);
    }
    println!();

    for i in (0..alpha.len()).rev() {
        println!("{}", i);
    }
    println!();

    for i in (1..alpha.len()+1).rev() {
        println!("{} {} {} ", i, i*2-2, i*2-1);
    }
}
