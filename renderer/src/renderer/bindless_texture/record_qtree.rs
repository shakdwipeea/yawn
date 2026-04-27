pub struct RecordQTree {
    q1: Option<Box<RecordQTree>>,
    q2: Option<Box<RecordQTree>>,
    q3: Option<Box<RecordQTree>>,
    q4: Option<Box<RecordQTree>>,
}

impl RecordQTree {
    pub fn new() -> Self {
        Self {
            q1: None,
            q2: None,
            q3: None,
            q4: None,
        }
    }

    pub fn get(&mut self, quadrant: u8) -> Option<&mut RecordQTree> {
        match quadrant {
            1 => self.q1.as_deref_mut(),
            2 => self.q2.as_deref_mut(),
            3 => self.q3.as_deref_mut(),
            4 => self.q4.as_deref_mut(),
            _ => None,
        }
    }

    pub fn delete(mut self: Self, quadrant: u8) {
        match quadrant {
            1 => {
                self.q1 = None;
            }
            2 => {
                self.q2 = None;
            }
            3 => {
                self.q3 = None;
            }
            4 => {
                self.q4 = None;
            }
            _ => {}
        };
    }

    pub fn is_free(self: Self, quadrant: u8) -> bool {
        match quadrant {
            1 => self.q1.is_none(),
            2 => self.q2.is_none(),
            3 => self.q3.is_none(),
            4 => self.q4.is_none(),
            _ => false,
        }
    }

    pub fn add(&mut self, quadrant: u8) {
        match quadrant {
            1 => {
                self.q1 = Some(Box::new(RecordQTree::new()));
            }
            2 => {
                self.q2 = Some(Box::new(RecordQTree::new()));
            }
            3 => {
                self.q3 = Some(Box::new(RecordQTree::new()));
            }
            4 => {
                self.q4 = Some(Box::new(RecordQTree::new()));
            }
            _ => {}
        }
    }
}
