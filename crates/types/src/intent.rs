/// Intent encoding matching the WGSL bit layout.
///
/// Intent word (u32):
///   [0:2]  target_direction (3 bits, 0-6)
///   [3:5]  action_type (3 bits, 0-5)
///   [6:31] bid (26 bits)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Direction {
    PosX = 0,
    NegX = 1,
    PosY = 2,
    NegY = 3,
    PosZ = 4,
    NegZ = 5,
    Self_ = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ActionType {
    NoAction = 0,
    Die = 1,
    Predate = 2,
    Replicate = 3,
    Move = 4,
    Idle = 5,
}

impl ActionType {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::NoAction,
            1 => Self::Die,
            2 => Self::Predate,
            3 => Self::Replicate,
            4 => Self::Move,
            5 => Self::Idle,
            _ => Self::NoAction,
        }
    }
}

impl Direction {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::PosX,
            1 => Self::NegX,
            2 => Self::PosY,
            3 => Self::NegY,
            4 => Self::PosZ,
            5 => Self::NegZ,
            6 => Self::Self_,
            _ => Self::Self_,
        }
    }
}

/// Encode an intent into a single u32.
pub fn intent_encode(action: ActionType, direction: Direction, bid: u32) -> u32 {
    let dir_bits = (direction as u32) & 0x7;
    let action_bits = ((action as u32) & 0x7) << 3;
    let bid_bits = (bid & 0x03FF_FFFF) << 6;
    dir_bits | action_bits | bid_bits
}

/// Decode an intent u32 into (ActionType, Direction, bid).
pub fn intent_decode(word: u32) -> (ActionType, Direction, u32) {
    let dir = Direction::from_u8((word & 0x7) as u8);
    let action = ActionType::from_u8(((word >> 3) & 0x7) as u8);
    let bid = (word >> 6) & 0x03FF_FFFF;
    (action, dir, bid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_all_actions() {
        let actions = [
            ActionType::NoAction,
            ActionType::Die,
            ActionType::Predate,
            ActionType::Replicate,
            ActionType::Move,
            ActionType::Idle,
        ];
        for action in actions {
            let word = intent_encode(action, Direction::PosX, 42);
            let (a, d, b) = intent_decode(word);
            assert_eq!(a, action);
            assert_eq!(d, Direction::PosX);
            assert_eq!(b, 42);
        }
    }

    #[test]
    fn bid_range() {
        // Minimum bid (0)
        let word = intent_encode(ActionType::Replicate, Direction::NegZ, 0);
        let (_, _, bid) = intent_decode(word);
        assert_eq!(bid, 0);

        // Maximum 26-bit bid
        let max_bid: u32 = 0x03FF_FFFF;
        let word = intent_encode(ActionType::Move, Direction::Self_, max_bid);
        let (a, d, b) = intent_decode(word);
        assert_eq!(a, ActionType::Move);
        assert_eq!(d, Direction::Self_);
        assert_eq!(b, max_bid);
    }

    #[test]
    fn direction_all_values() {
        let dirs = [
            Direction::PosX,
            Direction::NegX,
            Direction::PosY,
            Direction::NegY,
            Direction::PosZ,
            Direction::NegZ,
            Direction::Self_,
        ];
        for dir in dirs {
            let word = intent_encode(ActionType::Idle, dir, 100);
            let (_, d, _) = intent_decode(word);
            assert_eq!(d, dir);
        }
    }
}
