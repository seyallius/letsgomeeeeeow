#[derive(Debug, Clone)]
pub struct StationStats {
    pub min: i16,
    pub sum: i64,
    pub count: usize,
    pub max: i16,
}

impl StationStats {
    pub fn new(temperature: i16) -> Self {
        Self {
            min: temperature,
            sum: i64::from(temperature),
            count: 1,
            max: temperature,
        }
    }

    pub fn merge(&mut self, other: &StationStats) {
        self.min = self.min.min(other.min);
        self.sum += other.sum;
        self.count += other.count;
        self.max = self.max.max(other.max);
    }

    pub fn update(&mut self, temperature: i16) {
        self.min = self.min.min(temperature);
        self.sum += i64::from(temperature);
        self.count += 1;
        self.max = self.max.max(temperature);
    }
}
