#![deny(clippy::borrow_interior_mutable_const)]
#![allow(
    clippy::declare_interior_mutable_const,
    clippy::out_of_bounds_indexing,
    const_item_mutation
)]

use core::cell::{Cell, UnsafeCell};
use core::ops::{Deref, Index};

trait ConstDefault {
    const DEFAULT: Self;
}
impl ConstDefault for u32 {
    const DEFAULT: Self = 0;
}
impl<T: ConstDefault> ConstDefault for Cell<T> {
    const DEFAULT: Self = Cell::new(T::DEFAULT);
}

fn main() {
    {
        const C: String = String::new();
        let _ = C;
        let _ = &C;
        let _ = C.len();
        let _ = &*C;
    }
    {
        const C: UnsafeCell<u32> = UnsafeCell::new(0);
        let _ = C;
        let _ = &C; //~ borrow_interior_mutable_const
        let _ = C.into_inner();
        let _ = C.get(); //~ borrow_interior_mutable_const
    }
    {
        const C: Cell<u32> = Cell::new(0);
        let _ = C;
        let _ = &C; //~ borrow_interior_mutable_const
        let _ = &mut C; //~ borrow_interior_mutable_const
        let _ = C.into_inner();

        let local = C;
        C.swap(&local) //~ borrow_interior_mutable_const
    }
    {
        const C: [(Cell<u32>,); 1] = [(Cell::new(0),)];
        let _ = C;
        let _ = &C; //~ borrow_interior_mutable_const
        let _ = &C[0]; //~ borrow_interior_mutable_const
        let _ = &C[0].0; //~ borrow_interior_mutable_const
        C[0].0.set(1); //~ borrow_interior_mutable_const
    }
    {
        struct S(Cell<u32>);
        impl S {
            const C: Self = Self(Cell::new(0));
        }
        impl Deref for S {
            type Target = Cell<u32>;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        let _ = S::C;
        let _ = S::C.0;
        let _ = &S::C; //~ borrow_interior_mutable_const
        let _ = &S::C.0; //~ borrow_interior_mutable_const
        S::C.set(1); //~ borrow_interior_mutable_const
        let _ = &*S::C; //~ borrow_interior_mutable_const
        (*S::C).set(1); //~ borrow_interior_mutable_const
    }
    {
        enum E {
            Cell(Cell<u32>),
            Other,
        }
        const CELL: E = E::Cell(Cell::new(0));
        const OTHER: E = E::Other;

        let _ = CELL;
        let _ = &CELL; //~ borrow_interior_mutable_const
        let E::Cell(_) = CELL else {
            return;
        };

        let _ = OTHER;
        let _ = &OTHER;
        let E::Cell(ref _x) = OTHER else {
            return;
        };
    }
    {
        struct S<T> {
            cell: (Cell<T>, u32),
            other: Option<T>,
        }
        impl<T: ConstDefault + Copy> S<T> {
            const C: Self = Self {
                cell: (Cell::<T>::DEFAULT, 0),
                other: Some(T::DEFAULT),
            };

            fn f() {
                let _ = Self::C;
                let _ = &Self::C; //~ borrow_interior_mutable_const
                let _ = Self::C.other;
                let _ = &Self::C.other;
                let _ = &Self::C.cell; //~ borrow_interior_mutable_const
                let _ = &Self::C.cell.0; //~ borrow_interior_mutable_const
                Self::C.cell.0.set(T::DEFAULT); //~ borrow_interior_mutable_const
                let _ = &Self::C.cell.1;
            }
        }
    }
    {
        trait T {
            const VALUE: Option<Cell<u32>> = Some(Cell::new(0));
        }
        impl T for u32 {}
        impl T for i32 {
            const VALUE: Option<Cell<u32>> = None;
        }

        let _ = &u32::VALUE; //~ borrow_interior_mutable_const
        let _ = &i32::VALUE;
    }
    {
        trait Trait<T: ConstDefault> {
            type T<U: ConstDefault>: ConstDefault;
            const VALUE: Option<Self::T<T>> = Some(Self::T::<T>::DEFAULT);
        }
        impl<T: ConstDefault> Trait<T> for u32 {
            type T<U: ConstDefault> = Cell<U>;
        }
        impl<T: ConstDefault> Trait<T> for i32 {
            type T<U: ConstDefault> = Cell<U>;
            const VALUE: Option<Cell<T>> = None;
        }

        fn f<T: ConstDefault>() {
            let _ = &<u32 as Trait<T>>::VALUE; //~ borrow_interior_mutable_const
            let _ = &<i32 as Trait<T>>::VALUE;
        }
    }
    {
        struct S([Option<Cell<u32>>; 2]);
        impl Index<usize> for S {
            type Output = Option<Cell<u32>>;
            fn index(&self, idx: usize) -> &Self::Output {
                &self.0[idx]
            }
        }

        const C: S = S([Some(Cell::new(0)), None]);
        let _ = &C; //~ borrow_interior_mutable_const
        let _ = &C[0]; //~ borrow_interior_mutable_const
        let _ = &C.0[0]; //~ borrow_interior_mutable_const
        let _ = &C.0[1];
    }
    {
        const C: [Option<Cell<u32>>; 2] = [None, None];
        let _ = &C[0];
        let _ = &C[1];
        let _ = &C[2];

        fn f(i: usize) {
            let _ = &C[i];
        }
    }
    {
        const C: [Option<Cell<u32>>; 2] = [None, Some(Cell::new(0))];
        let _ = &C[0];
        let _ = &C[1]; //~ borrow_interior_mutable_const
        let _ = &C[2];

        fn f(i: usize) {
            let _ = &C[i]; //~ borrow_interior_mutable_const
        }
    }
}
