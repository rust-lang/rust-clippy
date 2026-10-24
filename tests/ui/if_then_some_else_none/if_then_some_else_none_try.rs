#![feature(try_blocks)]
#![warn(clippy::if_then_some_else_none)]
#![expect(clippy::needless_question_mark)]

use std::hint::black_box;

fn question_mark_in_some(cond: bool) -> Option<Option<u32>> {
    try { if cond { Some(black_box(Some(1))?) } else { None } }
}

fn question_mark_in_statement(cond: bool) -> Option<Option<u32>> {
    try {
        if cond {
            let value = black_box(Some(1))?;
            Some(value)
        } else {
            None
        }
    }
}

fn no_question_mark(cond: bool) -> Option<Option<u32>> {
    try {
        if cond { Some(1) } else { None }
        //~^ if_then_some_else_none
    }
}

fn question_mark_in_nested_closure(cond: bool) -> Option<Option<Option<u32>>> {
    try {
        if cond {
            Some((|| Some(black_box(Some(1))?))())
        } else {
            None
        }
        //~^^^^^ if_then_some_else_none
    }
}

fn question_mark_in_nested_try(cond: bool) -> Option<Option<Option<u32>>> {
    try {
        if cond { Some(try { black_box(Some(1))? }) } else { None }
        //~^ if_then_some_else_none
    }
}

fn question_mark_in_multiple_nested_tries(cond: bool) -> Option<Option<Option<u32>>> {
    try {
        if cond {
            Some(try { (try { black_box(Some(1))? })? })
        } else {
            None
        }
        //~^^^^^ if_then_some_else_none
    }
}

fn main() {}
