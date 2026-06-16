use engine::Commit;

fn main() {
    // let line = "work:report finished the Q2 draft -t 120";
    let line = "work:report finished the Q2 draft -t 120";
    match Commit::parse(line) {
        Ok(commit) => println!("{:#?}", commit),
        Err(e) => println!("couldn't parse that: {:?}", e),
    }
}
