#[derive(Debug, Clone, PartialEq)]
struct IntervalNode {
    start: f32,
    end: f32,
    left: Option<Box<IntervalNode>>,
    right: Option<Box<IntervalNode>>
}

impl IntervalNode {
    pub fn new(start: f32, end: f32) -> Self {
        Self {
            start: 0.0,
            end: 0.0,
            left: None,
            right: None
        }
    }

    pub fn find(&self, time: f32) -> Option<Box<IntervalNode>> {
        if time >= self.start && time <= self.end {
            Some(Box::new(self.clone()))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::interval_thread::IntervalNode;

    #[test]
    fn should_find_node_current() {
        let root = IntervalNode::new(0.0,10.0);
        let result = root.find(0.0);

        assert_ne!(result, None);
    }
}