use alloc::vec;

/// ModifyType for DeadlockDetector
pub enum ModifyType {
    /// down
    Allocate,
    /// up
    Release,
}

/// DeadlockDetector struct to implement the detection
pub struct DeadlockDetector<const MAX_RESOURCE_COUNT: usize, const MAX_THREAD_COUNT: usize> {
    /// keeps available resource count
    pub available: [usize; MAX_RESOURCE_COUNT],
    /// [i, j] = g, keeps the count g that thread[i] obsess resource[j]
    allocation: [[usize; MAX_RESOURCE_COUNT]; MAX_THREAD_COUNT],
    /// [i, j] = g, keeps the count g of resource[j]  that thread[i] still want
    need: [[usize; MAX_RESOURCE_COUNT]; MAX_THREAD_COUNT],
}

impl<const MAX_RESOURCE_COUNT: usize, const MAX_THREAD_COUNT: usize> Default
    for DeadlockDetector<MAX_RESOURCE_COUNT, MAX_THREAD_COUNT>
{
    fn default() -> Self {
        Self {
            available: [0; MAX_RESOURCE_COUNT],
            allocation: [[0; MAX_RESOURCE_COUNT]; MAX_THREAD_COUNT],
            need: [[0; MAX_RESOURCE_COUNT]; MAX_THREAD_COUNT],
        }
    }
}
impl<const MAX_RESOURCE_COUNT: usize, const MAX_THREAD_COUNT: usize>
    DeadlockDetector<MAX_RESOURCE_COUNT, MAX_THREAD_COUNT>
{
    /// create a new detector
    pub fn new() -> Self {
        Self::default()
    }

    /// modify the detector when up or down
    pub fn modify_resource(&mut self, modify_type: ModifyType, resource_id: usize, tid: usize) {
        match modify_type {
            ModifyType::Allocate => {
                if self.available[resource_id] == 0 {
                    self.need[tid][resource_id] += 1;
                    return;
                }

                self.allocation[tid][resource_id] += 1;
                self.available[resource_id] -= 1;
            }

            ModifyType::Release => {
                self.allocation[tid][resource_id] -= 1;
                self.available[resource_id] += 1;
            }
        }
    }

    /// run detection
    pub fn detect_deadlock(&mut self) -> bool {
        let mut work = self.available.to_vec();
        let mut finish = vec![false; self.available.len()];

        loop {
            for i in 0..finish.len() {
                if !finish[i] {
                    let mut is_satisfied = true;
                    for j in 0..self.available.len() {
                        if self.need[i][j] > work[j] {
                            is_satisfied = false;
                            break;
                        }
                    }
                    if is_satisfied {
                        finish[i] = true;
                        for z in 0..self.available.len() {
                            work[z] += self.allocation[i][z];
                        }
                    }
                } else if i == (finish.len() - 1) {
                    return !finish.iter().all(|&x| x);
                }
            }
        }
    }
}
