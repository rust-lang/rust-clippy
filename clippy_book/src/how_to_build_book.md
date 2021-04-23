# Guide for how to build the book

## Build the book
After clone a copy of the rust_clippy, type below to build the book.
```
mdbook build -o clippy_book/
```

notes:
- `-o` used to open the book after build it.
- build files html, css, and js files will be generated in `clippy_book` directory.

## Clean the book
To clean the extra files html, css, and js to clean the `clippy_book` directory, type below:
```
mdbook clean clippy_book/
```