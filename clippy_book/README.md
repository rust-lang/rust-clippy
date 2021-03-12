# Clippy Book guide

This is a guide for how to build the Clippy Book to be able to navigate it by browser.<br>
<br>
You need to meet below requirments:<br>
1. Install `mdbook` to build the book.<br>
Note: You can follow official `mdbook` guide for more details as per [link](https://rust-lang.github.io/mdBook/).
```bash
cargo install mdbook
```

2. Build the book and read it.<br>
Note: You need to target the book directory to build the required book.
```bash
cd rust-clippy
mdbook build -o clippy_book
```
Note that `-o` to open the book after build it.
