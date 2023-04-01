use std::sync::Arc;

use tokio::sync::{AcquireError, Semaphore};

#[derive(Debug)]
pub struct BoundedSemaphore {
    sem: Semaphore,
    max_permits: usize,
}

impl BoundedSemaphore {
    pub fn new(max_permits: usize, initial_permits: usize) -> Self {
        BoundedSemaphore {
            sem: Semaphore::new(initial_permits),
            max_permits,
        }
    }

    pub async fn acquire(&self) -> Result<BoundedSemaphorePermit<'_>, AcquireError> {
        let permit = self.sem.acquire().await?;
        permit.forget();
        Ok(BoundedSemaphorePermit {
            sem: self,
            permits: 1,
        })
    }

    pub async fn acquire_forget(&self) -> Result<BoundedSemaphorePermit<'_>, AcquireError> {
        let permit = self.sem.acquire().await?;
        permit.forget();
        Ok(BoundedSemaphorePermit {
            sem: self,
            permits: 0,
        })
    }

    pub async fn acquire_owned(
        self: Arc<Self>,
    ) -> Result<OwnedBoundedSemaphorePermit, AcquireError> {
        let permit = self.sem.acquire().await?;
        permit.forget();
        Ok(OwnedBoundedSemaphorePermit {
            sem: self,
            permits: 0,
        })
    }

    pub async fn acquire_forget_owned(
        self: Arc<Self>,
    ) -> Result<OwnedBoundedSemaphorePermit, AcquireError> {
        let permit = self.sem.acquire().await?;
        permit.forget();
        Ok(OwnedBoundedSemaphorePermit {
            sem: self,
            permits: 0,
        })
    }

    pub fn add_permits(&self, n: usize) {
        let permits = self.sem.available_permits();
        let new_permits = std::cmp::min(permits + n, self.max_permits);
        let diff = new_permits - permits;
        if diff > 0 {
            self.sem.add_permits(diff);
        }
    }
}

pub struct BoundedSemaphorePermit<'a> {
    sem: &'a BoundedSemaphore,
    permits: usize,
}

impl<'a> Drop for BoundedSemaphorePermit<'a> {
    fn drop(&mut self) {
        self.sem.add_permits(self.permits);
    }
}

pub struct OwnedBoundedSemaphorePermit {
    sem: Arc<BoundedSemaphore>,
    permits: usize,
}

impl Drop for OwnedBoundedSemaphorePermit {
    fn drop(&mut self) {
        self.sem.add_permits(self.permits);
    }
}
