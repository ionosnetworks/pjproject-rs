use pjproject_sys as pj;

pub fn pj_rand() -> i32 {
    unsafe { pj::pj_rand() }
}

pub fn pj_srand(seed: u32) {
    unsafe { pj::pj_srand(seed) }
}
