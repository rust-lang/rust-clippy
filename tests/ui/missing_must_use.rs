#![warn(clippy::missing_must_use)]

pub struct PubStructUnitNoMustUse;
//~^ missing_must_use
struct PrivStructUnitNoMustUse;
//~^ missing_must_use

pub struct PubTupleStructNoMustUse(u8);
//~^ missing_must_use
struct PrivTupleStructNoMustUse(u8);
//~^ missing_must_use

pub struct PubStructNoMustUse {
    //~^ missing_must_use
    pub field: u8,
}
struct PrivStructNoMustUse {
    //~^ missing_must_use
    pub field: u8,
}

pub enum PubEnumNoMustUse {
    //~^ missing_must_use
    Unit,
    Tuple(u8),
    Struct { field: u8 },
}
enum PrivEnumNoMustUse {
    //~^ missing_must_use
    Unit,
    Tuple(u8),
    Struct { field: u8 },
}

pub union PubUnionNoMustUse {
    //~^ missing_must_use
    f1: u8,
    f2: u16,
}
union PrivUnionNoMustUse {
    //~^ missing_must_use
    f1: u8,
    f2: u16,
}

#[must_use]
pub struct PubStructUnitMustUse;
#[must_use]
struct PrivStructUnitMustUse;

#[must_use]
pub struct PubStructTupleMustUse(u8);
#[must_use]
struct PrivStructTupleMustUse(u8);

#[must_use]
pub struct PubStructMustUse {
    pub field: u8,
}
#[must_use]
struct PrivStructMustUse {
    pub field: u8,
}

#[must_use]
pub enum PubEnumMustUse {
    Unit,
    Tuple(u8),
    Struct { field: u8 },
}
#[must_use]
enum PrivEnumMustUse {
    Unit,
    Tuple(u8),
    Struct { field: u8 },
}

#[must_use]
pub union PubUnionMustUse {
    f1: u8,
    f2: u16,
}
#[must_use]
union PrivUnionMustUse {
    f1: u8,
    f2: u16,
}

fn main() {
    pub struct InnerPubStructUnitNoMustUse;
    //~^ missing_must_use
    struct InnerPrivStructUnitNoMustUse;
    //~^ missing_must_use

    pub struct InnerPubStructTupleNoMustUse(u8);
    //~^ missing_must_use
    struct InnerPrivStructTupleNoMustUse(u8);
    //~^ missing_must_use

    pub struct InnerPubStructNoMustUse {
        //~^ missing_must_use
        pub field: u8,
    }
    struct InnerPrivStructNoMustUse {
        //~^ missing_must_use
        pub field: u8,
    }

    pub enum InnerPubEnumNoMustUse {
        //~^ missing_must_use
        Unit,
        Tuple(u8),
        Struct { field: u8 },
    }
    enum InnerPrivEnumNoMustUse {
        //~^ missing_must_use
        Unit,
        Tuple(u8),
        Struct { field: u8 },
    }

    pub union InnerPubUnionNoMustUse {
        //~^ missing_must_use
        f1: u8,
        f2: u16,
    }
    union InnerPrivUnionNoMustUse {
        //~^ missing_must_use
        f1: u8,
        f2: u16,
    }

    #[must_use]
    pub struct InnerPubStructUnitMustUse;
    #[must_use]
    struct InnerPrivStructUnitMustUse;

    #[must_use]
    pub struct InnerPubStructTupleMustUse(u8);
    #[must_use]
    struct InnerPrivStructTupleMustUse(u8);

    #[must_use]
    pub struct InnerPubStructMustUse {
        pub field: u8,
    }
    #[must_use]
    struct InnerPrivStructMustUse {
        pub field: u8,
    }

    #[must_use]
    pub enum InnerPubEnumMustUse {
        Unit,
        Tuple(u8),
        Struct { field: u8 },
    }
    #[must_use]
    enum InnerPrivEnumMustUse {
        Unit,
        Tuple(u8),
        Struct { field: u8 },
    }

    #[must_use]
    pub union InnerPubUnionMustUse {
        f1: u8,
        f2: u16,
    }
    #[must_use]
    union InnerPrivUnionMustUse {
        f1: u8,
        f2: u16,
    }
}
