mod analyzer;
mod article;

fn main() {
    println!("Hello, world!");
    let mut analyzer = analyzer::Analyzer::new(0.1, 0.2);
    analyzer.run();
}
