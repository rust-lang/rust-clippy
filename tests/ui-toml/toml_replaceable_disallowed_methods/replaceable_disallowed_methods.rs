fn bad() {}
fn questionable() {}
fn good() {}
fn free(_: WithMethod) {}

pub struct WithMethod;

impl WithMethod {
    pub fn evil(self) {}
    pub fn mean(self) {}
    pub fn ugly(self) {}
    pub fn nice(self) {}
    pub fn cruel(self, _: i32) {}
    pub fn kind(self, _: i32) {}
}

fn main() {
    bad();
    //~^ disallowed_methods
    questionable();
    //~^ disallowed_methods
    3f64.round();
    //~^ disallowed_methods
    WithMethod.evil();
    //~^ disallowed_methods
    WithMethod::evil(WithMethod);
    //~^ disallowed_methods
    WithMethod.mean();
    //~^ disallowed_methods
    WithMethod::mean(WithMethod);
    //~^ disallowed_methods
    WithMethod.ugly();
    //~^ disallowed_methods
    WithMethod::ugly(WithMethod);
    //~^ disallowed_methods
    WithMethod.cruel(1);
    //~^ disallowed_methods
    WithMethod::cruel(WithMethod, 1);
    //~^ disallowed_methods

    WithMethod.nice();
    good();
    free(WithMethod);
}
