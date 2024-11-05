use std::{ffi::CStr, marker::PhantomData, pin::Pin};

use pjproject_sys as pj;

pub const PJ_CACHING_POOL_DEAULT_INIT_SIZE: usize = 1000;
pub const PJ_CACHING_POOL_DEAULT_INCR_SIZE: usize = 1000;

pub struct PjPool {
    pool: PjPoolRef<'static>,
    #[allow(dead_code)]
    caching_pool: PjCachingPool,
}

unsafe impl Send for PjPool {}
unsafe impl Sync for PjPool {}

impl PjPool {
    pub fn new<S: AsRef<CStr>>(
        mut caching_pool: PjCachingPool,
        name: S,
        initial_size: usize,
        increment_size: usize,
    ) -> Self {
        let name = name.as_ref();
        let name = name.as_ptr() as *const i8;
        let pool = unsafe {
            pj::pj_pool_create(
                caching_pool.factory_mut().as_mut(),
                name,
                initial_size,
                increment_size,
                None,
            )
        };

        Self {
            pool: PjPoolRef::from(pool),
            caching_pool,
        }
    }

    pub fn as_ptr(&self) -> *const pj::pj_pool_t {
        self.pool.as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut pj::pj_pool_t {
        self.pool.as_mut_ptr()
    }

    pub fn default_with_name<S: AsRef<CStr>>(name: S) -> Self {
        Self::new(
            PjCachingPool::default(),
            name,
            PJ_CACHING_POOL_DEAULT_INIT_SIZE,
            PJ_CACHING_POOL_DEAULT_INCR_SIZE,
        )
    }
}

impl Drop for PjPool {
    fn drop(&mut self) {
        unsafe { pj::pj_pool_release(self.pool.as_mut_ptr()) };
    }
}

impl<'a> AsRef<PjPoolRef<'a>> for PjPool {
    #[inline]
    fn as_ref(&self) -> &PjPoolRef<'a> {
        &self.pool
    }
}

pub struct PjPoolRef<'a> {
    pool: *mut pj::pj_pool_t,
    phantom: PhantomData<&'a ()>,
}

impl<'a> PjPoolRef<'a> {
    pub fn as_ptr(&self) -> *const pj::pj_pool_t {
        self.pool
    }

    pub fn as_mut_ptr(&self) -> *mut pj::pj_pool_t {
        self.pool
    }

    pub fn as_ref(&self) -> &pj::pj_pool_t {
        unsafe { &*self.as_ptr() }
    }
}

impl<'a> From<*mut pj::pj_pool_t> for PjPoolRef<'a> {
    fn from(value: *mut pj::pj_pool_t) -> Self {
        Self {
            pool: value,
            phantom: PhantomData,
        }
    }
}

pub struct PjCachingPool {
    caching_pool: Pin<Box<pj::pj_caching_pool>>,
}

unsafe impl Send for PjCachingPool {}
unsafe impl Sync for PjCachingPool {}

impl PjCachingPool {
    fn new(policy: PjPoolFactoryPolicy, max_capacity: usize) -> Self {
        let mut caching_pool = Box::pin(unsafe { std::mem::zeroed::<pj::pj_caching_pool>() });

        unsafe {
            pj::pj_caching_pool_init(
                caching_pool.as_mut().get_mut() as *mut _,
                policy.as_ptr(),
                max_capacity,
            );
        };

        Self { caching_pool }
    }

    pub fn factory(&self) -> PjPoolFactory {
        PjPoolFactory {
            factory: &self.caching_pool.factory,
        }
    }

    pub fn factory_mut(&mut self) -> PjPoolFactoryMut {
        PjPoolFactoryMut {
            factory: &mut self.caching_pool.factory,
        }
    }
}

pub struct PjCachingPoolBuilder {
    policy: PjPoolFactoryPolicy,
    max_capacity: usize,
}

impl Default for PjCachingPool {
    fn default() -> Self {
        let mut ret = Self {
            caching_pool: Box::pin(unsafe { std::mem::zeroed::<pj::pj_caching_pool>() }),
        };
        let policy = unsafe { pj::pj_pool_factory_default_policy };

        unsafe {
            pj::pj_caching_pool_init(
                ret.caching_pool.as_mut().get_mut() as *mut _,
                &policy as *const _,
                0,
            )
        };

        ret
    }
}

impl PjCachingPoolBuilder {
    pub fn with_policy(&mut self, policy: PjPoolFactoryPolicy) -> &mut Self {
        self.policy = policy;
        self
    }

    pub fn with_max_capacity(&mut self, max_capacity: usize) -> &mut Self {
        self.max_capacity = max_capacity;
        self
    }

    pub fn build(self) -> PjCachingPool {
        PjCachingPool::new(self.policy, self.max_capacity)
    }
}

impl Drop for PjCachingPool {
    fn drop(&mut self) {
        unsafe { pj::pj_caching_pool_destroy(self.caching_pool.as_mut().get_mut() as *mut _) };
    }
}

impl Default for PjCachingPoolBuilder {
    fn default() -> Self {
        Self {
            policy: Default::default(),
            max_capacity: Default::default(),
        }
    }
}

pub struct PjPoolFactory<'a> {
    #[allow(dead_code)]
    factory: &'a pj::pj_pool_factory,
}

pub struct PjPoolFactoryMut<'a> {
    factory: &'a mut pj::pj_pool_factory,
}

impl<'a> PjPoolFactoryMut<'a> {
    pub fn as_mut(&mut self) -> &mut pj::pj_pool_factory {
        self.factory
    }
}

pub struct PjPoolFactoryPolicy(pj::pj_pool_factory_policy);

impl PjPoolFactoryPolicy {
    pub fn as_ptr(&self) -> *const pj::pj_pool_factory_policy {
        &self.0
    }
}

impl Default for PjPoolFactoryPolicy {
    fn default() -> Self {
        Self(unsafe { pj::pj_pool_factory_default_policy })
    }
}
