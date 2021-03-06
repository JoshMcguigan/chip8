extern crate bit_vec;
extern crate rand;

use bit_vec::BitVec;
use rand::Rng;
use std::fmt;
use std::num::Wrapping;

/// The size of the chip's memory (RAM and ROM storage).
const NMEM: usize = 4096;

/// The number of registers.
const NREG: usize = 16;

/// The width of the display (in pixels)
pub const WIDTH: u32 = 64;

/// The height of the display (in pixels)
pub const HEIGHT: u32 = 32;

/// The total number of pixels.
pub const NPIXELS: usize = (WIDTH * HEIGHT) as usize;

/*
 * From http://www.multigesture.net/articles/how-to-write-an-emulator-chip-8-interpreter/
 * MEMORY MAP:
 * 0x000-0x1FF: Chip 8 interpreter
 * 0x050-0x0A0: 4x5 pixel font set (0-F)
 * 0x200-0xFFF: Program ROM and RAM
 */
/// The built in fonts that are loaded into memory during initialization.
static FONTSET: [u8;80] = [
  0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
  0x20, 0x60, 0x20, 0x20, 0x70, // 1
  0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
  0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
  0x90, 0x90, 0xF0, 0x10, 0x10, // 4
  0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
  0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
  0xF0, 0x10, 0x20, 0x40, 0x40, // 7
  0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
  0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
  0xF0, 0x90, 0xF0, 0x90, 0x90, // A
  0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
  0xF0, 0x80, 0x80, 0x80, 0xF0, // C
  0xE0, 0x90, 0x90, 0x90, 0xE0, // D
  0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
  0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

/// Returns a string for the version of the library.
pub fn version() -> &'static str {
    concat!(env!("CARGO_PKG_VERSION_MAJOR"),
    ".",
    env!("CARGO_PKG_VERSION_MINOR"),
    ".",
    env!("CARGO_PKG_VERSION_PATCH")
    )
}

/// The Chip8 emulator.  This can load vectors of `u8` representations of ROMs
/// and play them.
pub struct Chip8 {
    pub draw_flag: bool,
    opcode: u16,
    memory: [u8; NMEM],
    reg: [u8; NREG],
    index: u16,
    pc: u16,
    pub graphics: [bool; NPIXELS],
    timer_delay: u8,
    timer_sound: u8,
    stack: [u16; 16],
    sp: u16,
    pub key: [u8; 16],
    pub make_sound: bool,
}

impl fmt::Debug for Chip8 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "Chip8:
                 draw_flag: {}
                 opcode: {:#X}
                 index: {:#X}
                 pc: {:#X}
                 t_delay: {:#X}
                 t_sound: {:#X}
                 r0: {:#X}  r1: {:#X}  r2: {:#X} r3: {:#X}
                 r4: {:#X}  r5: {:#X}  r6: {:#X} r7: {:#X}
                 r8: {:#X}  r9: {:#X}  rA: {:#X} rB: {:#X}
                 rC: {:#X}  rD: {:#X}  rE: {:#X} rF: {:#X}
                 sp: {:#X}  stack: {:?}\n",
               self.draw_flag,
               self.opcode,
               self.index,
               self.pc,
               self.timer_delay,
               self.timer_sound,
               self.reg[0],
               self.reg[1],
               self.reg[2],
               self.reg[3],
               self.reg[4],
               self.reg[5],
               self.reg[6],
               self.reg[7],
               self.reg[8],
               self.reg[9],
               self.reg[10],
               self.reg[11],
               self.reg[12],
               self.reg[13],
               self.reg[14],
               self.reg[15],
               self.sp,
               self.stack,
               ).unwrap();
        write!(f, "  graphics:\n").unwrap();
        let mut row = String::with_capacity(64);
        write!(f, "+----------------------------------------------------------------+\n").unwrap();
        for column_index in 0..32 {
            for row_index in 0..64 {
                row.push(if self.graphics[column_index * 64 + row_index] { '#' } else { ' ' });
            }
            write!(f, "|{}|\n", row).unwrap();
            row.clear();
        }
        write!(f, "+----------------------------------------------------------------+\n")
    }
}

impl Default for Chip8 {
    /// Constructs a new Chip8 emulator.
    fn default() -> Self {
        let mut chip = Chip8 {
            draw_flag: true,
            opcode: 0,
            memory: [0; NMEM],
            reg: [0; NREG],
            index: 0,
            pc: 0x200,
            graphics: [false; NPIXELS],
            timer_delay: 0,
            timer_sound: 0,
            stack: [0; 16],
            sp: 0,
            key: [0; 16],
            make_sound: false,
        };

        // Initialize the font set
        for (i, item) in FONTSET.iter().enumerate().take(80) {
            chip.memory[i] = *item;
        }

        chip
    }
}

impl Chip8 {
    /// Loads the given bytes into the chip's memory.
    pub fn load(&mut self, game: &[u8]) {
        for (i, byte) in game.iter().enumerate() {
            self.memory[i + 0x200] = *byte;
        }
    }

    /// Run the emulator through a single cycle.
    /// # Panics
    /// If the emulator comes across an invalid opcode, it will panic with a
    /// description of the error (including the invalid opcode).
    pub fn emulate_cycle(&mut self) {
        // Fetch opcode
        self.fetch_opcode();

        // Decode and Execute opcode
        self.execute_opcode();

        // Update timers
        self.update_timers();
    }

    /// Read the next opcode from memory.
    fn fetch_opcode(&mut self) {
        self.opcode = (self.memory[self.pc as usize] as u16) << 8 |
            self.memory[(self.pc + 1) as usize] as u16;
    }

    /// Run the current opcode, storing the results in the chip.
    /// # Panics
    fn execute_opcode(&mut self) {
        match self.opcode & 0xF000 {
            0x0000 => {
                match self.opcode {
                    0x00E0 => {
                        // 0x00E0: Clears the screen
                        for i in 0..NPIXELS {
                            self.graphics[i] = false;
                        }
                        self.draw_flag = true;
                        self.pc += 2;
                    }
                    0x00EE => {
                        // 0x00EE: Return from subroutine
                        self.sp -= 1;
                        self.pc = self.stack[self.sp as usize];
                        self.pc += 2;
                    }
                    _ => panic!("Opcode {:#X} is bad", self.opcode),
                }
            }
            0x1000 => {
                // 0x1NNN: Jump to address NNN
                self.pc = self.opcode & 0x0FFF;
            }
            0x2000 => {
                // 0x2NNN: Call subroutine at NNN
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = self.opcode & 0x0FFF;
            },
            0x3000 => {
                // 0x3XNN: Skip next instruction if regX equals NN
                let x = (self.opcode & 0x0F00) >> 8;
                let nn = (self.opcode & 0x00FF) as u8;
                if self.reg[x as usize] == nn {
                    self.pc += 2;
                }
                self.pc += 2;
            },
            0x4000 => {
                // 0x4XNN: Skip next instruction if regX does not equal NN
                let x = (self.opcode & 0x0F00) >> 8;
                let nn = (self.opcode & 0x00FF) as u8;
                if self.reg[x as usize] != nn {
                    self.pc += 2;
                }
                self.pc += 2;
            },
            0x5000 => {
                // 0x5XY0: Skip next instruction if regX equals regY
                let x = (self.opcode & 0x0F00) >> 8;
                let y = (self.opcode & 0x00F0) >> 4;
                if self.reg[x as usize] == self.reg[y as usize] {
                    self.pc += 2;
                }
                self.pc += 2;
            },
            0x6000 => {
                // 0x6XNN: Set regX to NN
                let x = (self.opcode & 0x0F00) >> 8;
                let nn = (self.opcode & 0x00FF) as u8;
                self.reg[x as usize] = nn;
                self.pc += 2;
            },
            0x7000 => {
                // 0x7XNN: Add NN to regX
                let x = (self.opcode & 0x0F00) >> 8;
                let nn = Wrapping((self.opcode & 0x00FF) as u8);
                let x_val = Wrapping(self.reg[x as usize]);
                self.reg[x as usize] = (x_val + nn).0;
                self.pc += 2;
            },
            0x8000 => {
                match self.opcode & 0x000F {
                    0x0000 => {
                        // 0x8XY0: Set regX to regY
                        let x = (self.opcode & 0x0F00) >> 8;
                        let y = (self.opcode & 0x00F0) >> 4;
                        self.reg[x as usize] = self.reg[y as usize];
                        self.pc += 2;
                    }
                    0x0001 => {
                        // 0x8XY1: Set regX to regX | regY
                        let x = (self.opcode & 0x0F00) >> 8;
                        let y = (self.opcode & 0x00F0) >> 4;
                        self.reg[x as usize] |= self.reg[y as usize];
                        self.pc += 2;
                    }
                    0x0002 => {
                        // 0x8XY2: Set regX to regX & regY
                        let x = (self.opcode & 0x0F00) >> 8;
                        let y = (self.opcode & 0x00F0) >> 4;
                        self.reg[x as usize] &= self.reg[y as usize];
                        self.pc += 2;
                    }
                    0x0003 => {
                        // 0x8XY3: Set regX to regX ^ regY
                        let x = (self.opcode & 0x0F00) >> 8;
                        let y = (self.opcode & 0x00F0) >> 4;
                        self.reg[x as usize] ^= self.reg[y as usize];
                        self.pc += 2;
                    }
                    0x0004 => {
                        // 0x8XY4: Add regY to regX, set carry if needed
                        if self.reg[((self.opcode & 0x00F0) >> 4) as usize] >
                            (0xFF - self.reg[((self.opcode & 0x0F00) >> 8) as usize]) {
                            self.reg[0xF] = 1;
                        } else {
                            self.reg[0xF] = 0;
                        }
                        let x = Wrapping(self.reg[((self.opcode & 0x0F00) >> 8) as usize]);
                        let y = Wrapping(self.reg[((self.opcode & 0x00F0) >> 4) as usize]);
                        self.reg[((self.opcode & 0x0F00) >> 8) as usize] = (x + y).0;
                        self.pc += 2;
                    }
                    0x0005 => {
                        // 0x8XY5: regX -= regY, regF = 0 if borrow, else 1
                        let x_index = ((self.opcode & 0x0F00) >> 8) as usize;
                        let y_index = ((self.opcode & 0x00F0) >> 4) as usize;
                        if self.reg[y_index] > self.reg[x_index] {
                            self.reg[0xF] = 0;
                        } else {
                            self.reg[0xF] = 1;
                        }
                        let x = Wrapping(self.reg[x_index]);
                        let y = Wrapping(self.reg[y_index]);
                        self.reg[x_index] = (x - y).0;
                        self.pc += 2;
                    }
                    0x0006 => {
                        // 0x8X_6: Shifts regX right by one, setting regF to lsb of regX before
                        let x = (self.opcode & 0x0F00) >> 8;
                        let lsb = (self.reg[x as usize] & 0x0001) as u8;
                        self.reg[0xF] = lsb;
                        self.reg[x as usize] >>= 1;
                        self.pc += 2;
                    }
                    0x0007 => {
                        // 0x8XY7: regX = regY - regX, regF = 0 if borrow, else 1
                        let x_index = ((self.opcode & 0x0F00) >> 8) as usize;
                        let y_index = ((self.opcode & 0x00F0) >> 4) as usize;
                        if self.reg[x_index] > self.reg[y_index] {
                            self.reg[0xF] = 0;
                        } else {
                            self.reg[0xF] = 1;
                        }
                        let x = Wrapping(self.reg[x_index]);
                        let y = Wrapping(self.reg[y_index]);
                        self.reg[x_index] = (y - x).0;
                        self.pc += 2;
                    }
                    0x000E => {
                        // 0x8X_E: Shifts regX left by one, setting regF to msb of regX before
                        let x = (self.opcode & 0x0F00) >> 8;
                        let msb = ((self.reg[x as usize] & 0x80) as u8) >> 7;
                        self.reg[0xF] = msb;
                        self.reg[x as usize] <<= 1;
                        self.pc += 2;
                    }
                    _ => panic!("Opcode {:#X} is bad", self.opcode),
                }
            },
            0x9000 => {
                // 0x9XY0: Skip next instruction if regX does not equal regY
                let x = (self.opcode & 0x0F00) >> 8;
                let y = (self.opcode & 0x00F0) >> 4;
                if self.reg[x as usize] != self.reg[y as usize] {
                    self.pc += 2;
                }
                self.pc += 2;
            },
            0xA000 => {
                // 0xANNN: Sets I to the address NNN
                self.index = self.opcode & 0x0FFF;
                self.pc += 2;
            },
            0xB000 => {
                // 0xBNNN: Jump to address NNN + reg0
                let address = self.opcode & 0x0FFF;
                self.pc = address + self.reg[0] as u16;
            }
            0xC000 => {
                // 0xCXNN: regX = random number & NN
                let x = (self.opcode & 0x0F00) >> 8;
                let nn = (self.opcode & 0x00FF) as u8;
                self.reg[x as usize] = nn & (rand::thread_rng().gen_range(0,255) as u8);
                self.pc += 2;
            }
            0xD000 => {
                // 0xDXYN: Draw sprint at regX,regY with N bytes of data, start at index
                // TODO: Fix wrapping of stuff
                let x = self.reg[((self.opcode & 0x0F00) >> 8) as usize] as i32;
                let y = self.reg[((self.opcode & 0x00F0) >> 4) as usize] as i32;
                let n = (self.opcode & 0x000F) as u16;
                let start = self.index;

                self.reg[0xF] = 0;
                for i in start..(start+n) {
                    let row_num = (i - start) as u8;
                    let bits = BitVec::from_bytes(&[self.memory[i as usize]]);
                    for j in 0..8 {
                        let x_s = x + j;
                        let y_s = y + row_num as i32;
                        if 0 <= x_s &&
                           x_s < 64 &&
                           0 <= y_s &&
                           y_s < 32 {
                               let address = (64 * y_s) + x_s;
                               if bits[j as usize] {
                                   if self.graphics[address as usize] {
                                       self.reg[0xF] = 1;
                                   }
                                   self.graphics[address as usize] ^= true;
                               }
                           }
                    }
                }
                self.draw_flag = true;
                self.pc += 2;
            }
            0xE000 => {
                match self.opcode & 0x00FF {
                    0x009E => {
                        // 0xEX9E: Skips next instruction if key store in regX is pressed
                        let x = (self.opcode & 0x0F00) >> 8;
                        if self.key[self.reg[x as usize] as usize] != 0 {
                            self.pc += 2;
                        }
                        self.pc += 2;
                    }
                    0x00A1 => {
                        // 0xEXA1: Skips next instruction if key store in regX is not pressed
                        let x = (self.opcode & 0x0F00) >> 8;
                        if self.key[self.reg[x as usize] as usize] == 0 {
                            self.pc += 2;
                        }
                        self.pc += 2;
                    }
                    _ => panic!("Opcode {:#X} is bad", self.opcode),
                }
            },
            0xF000 => {
                match self.opcode & 0x00FF {
                    0x0007 => {
                        // 0xFX07: Sets regX to the value of the delay timer
                        let x = (self.opcode & 0x0F00) >> 8;
                        self.reg[x as usize] = self.timer_delay;
                        self.pc += 2;
                    }
                    0x000A => {
                        // 0xFX0A: Wait for a keypress, halting operation, store in regX
                        let x = ((self.opcode & 0x0F00) >> 8) as usize;
                        for k in 0..0xF {
                            if self.key[k as usize] != 0 {
                                self.reg[x] = k as u8;
                                self.pc += 2;
                                break;
                            }
                        }
                    }
                    0x0015 => {
                        // 0xFX15: Sets delay timer to regX
                        let x = (self.opcode & 0x0F00) >> 8;
                        self.timer_delay = self.reg[x as usize];
                        self.pc += 2;
                    }
                    0x0018 => {
                        // 0xFX18: Sets sound timer to regX
                        let x = (self.opcode & 0x0F00) >> 8;
                        self.timer_sound = self.reg[x as usize];
                        self.pc += 2;
                    }
                    0x001E => {
                        // 0xFX1E: Add regX to index
                        let x = (self.opcode & 0x0F00) >> 8;
                        self.index += self.reg[x as usize] as u16;
                        self.pc += 2;
                    }
                    0x0029 => {
                        // 0xFX29: Sets index to location of character in regX
                        let x = (self.opcode & 0x0F00) >> 8;
                        self.index = 5 * self.reg[x as usize] as u16;
                        self.pc += 2;
                    }
                    0x0033 => {
                        // 0xFX33: Store binary coded decimal of regX
                        // http://www.multigesture.net/wp-content/uploads/mirror/goldroad/chip8.shtml
                        let x = ((self.opcode & 0x0F00) >> 8) as usize;
                        self.memory[self.index as usize] = self.reg[x] / 100;
                        self.memory[(self.index + 1) as usize] = (self.reg[x] / 10) % 10;
                        self.memory[(self.index + 2) as usize] = (self.reg[x] % 100) % 10;
                        self.pc += 2;
                    }
                    0x0055 => {
                        // 0xFX55: Stores reg0 through regX (inclusive) in memory starting at index
                        let x = ((self.opcode & 0x0F00) >> 8) as usize;
                        for i in 0..(x+1) {
                            self.memory[self.index as usize + i] = self.reg[i];
                        }
                        self.pc += 2;
                    }
                    0x0065 => {
                        // 0xFX65: Fills reg0 through regX (inclusive) from memory starting at index
                        let x = ((self.opcode & 0x0F00) >> 8) as usize;
                        for i in 0..(x+1) {
                            self.reg[i] = self.memory[self.index as usize + i];
                        }
                        self.pc += 2;
                    }
                    _ => panic!("Opcode {:#X} is bad", self.opcode),
                }
            },
            _ => panic!("Opcode {:#X} is bad", self.opcode),
        }
    }

    /// Update the chip's internal timers for delay and sound.
    fn update_timers(&mut self) {
        if self.timer_delay > 0 {
            self.timer_delay -= 1;
        }

        self.make_sound = false;
        if self.timer_sound > 0 {
            if self.timer_sound == 1 {
                self.make_sound = true;
            }
            self.timer_sound -= 1;
        }
    }
}

#[cfg(test)]
mod test {
    use super::Chip8;

    #[test]
    fn op_00e0() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x00, 0xE0]);
        chip.graphics[1] = true;
        assert_eq!(chip.pc, 512);

        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        assert_eq!(chip.graphics[1], false);
    }

    #[test]
    fn op_00ee() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x00, 0xEE]);
        chip.stack[0] = 0x42;
        chip.sp = 1;
        assert_eq!(chip.pc, 512);

        chip.emulate_cycle();
        assert_eq!(chip.pc, 0x44);
        assert_eq!(chip.sp, 0);
    }

    #[test]
    fn op_1nnn() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x16, 0x66]);
        assert_eq!(chip.pc, 512);
        chip.emulate_cycle();
        assert_eq!(chip.pc, 0x666);
    }

    #[test]
    fn op_2nnn() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x26, 0x66]);
        assert_eq!(chip.pc, 512);
        chip.emulate_cycle();
        assert_eq!(chip.pc, 0x666);
        assert_eq!(chip.stack[0], 512);
        assert_eq!(chip.sp, 1);
    }

    #[test]
    fn op_3xnn() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x31, 0x66, 0x31, 0x67]);
        chip.reg[1] = 0x67;
        assert_eq!(chip.pc, 512);
        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        chip.emulate_cycle();
        assert_eq!(chip.pc, 518);
    }

    #[test]
    fn op_4xnn() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x41, 0x66, 0x41, 0x67]);
        chip.reg[1] = 0x66;
        assert_eq!(chip.pc, 512);
        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        chip.emulate_cycle();
        assert_eq!(chip.pc, 518);
    }

    #[test]
    fn op_5xy0() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x51, 0x20, 0x51, 0x30]);
        chip.reg[1] = 0x66;
        chip.reg[2] = 0x22;
        chip.reg[3] = 0x66;
        assert_eq!(chip.pc, 512);
        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        chip.emulate_cycle();
        assert_eq!(chip.pc, 518);
    }

    #[test]
    fn op_6xnn() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x6A, 0x2F]);
        assert_eq!(chip.reg[0xA], 0);
        assert_eq!(chip.pc, 512);
        chip.emulate_cycle();
        assert_eq!(chip.reg[0xA], 0x2F);
        assert_eq!(chip.pc, 514);
    }

    #[test]
    fn op_7xnn() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x7A, 0x2F]);
        chip.reg[0xA] = 0xB;
        assert_eq!(chip.reg[0xA], 0xB);
        assert_eq!(chip.pc, 512);
        chip.emulate_cycle();
        assert_eq!(chip.reg[0xA], 0x2F + 0xB);
        assert_eq!(chip.pc, 514);
    }

    #[test]
    fn op_8xy0() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x8A, 0x20]);
        chip.reg[0xA] = 0xB;
        chip.reg[0x2] = 0xC;
        assert_eq!(chip.reg[0xA], 0xB);
        assert_eq!(chip.reg[0x2], 0xC);
        assert_eq!(chip.pc, 512);
        chip.emulate_cycle();
        assert_eq!(chip.reg[0xA], 0xC);
        assert_eq!(chip.reg[0x2], 0xC);
        assert_eq!(chip.pc, 514);
    }

    #[test]
    fn op_8xy1() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x8A, 0x21]);
        chip.reg[0xA] = 0xB;
        chip.reg[0x2] = 0xC;
        assert_eq!(chip.reg[0xA], 0xB);
        assert_eq!(chip.reg[0x2], 0xC);
        assert_eq!(chip.pc, 512);
        chip.emulate_cycle();
        assert_eq!(chip.reg[0xA], 0xB | 0xC);
        assert_eq!(chip.reg[0x2], 0xC);
        assert_eq!(chip.pc, 514);
    }

    #[test]
    fn op_8xy2() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x8A, 0x22]);
        chip.reg[0xA] = 0xB;
        chip.reg[0x2] = 0xC;
        assert_eq!(chip.reg[0xA], 0xB);
        assert_eq!(chip.reg[0x2], 0xC);
        assert_eq!(chip.pc, 512);
        chip.emulate_cycle();
        assert_eq!(chip.reg[0xA], 0xB & 0xC);
        assert_eq!(chip.reg[0x2], 0xC);
        assert_eq!(chip.pc, 514);
    }

    #[test]
    fn op_8xy3() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x8A, 0x23]);
        chip.reg[0xA] = 0xB;
        chip.reg[0x2] = 0xC;
        assert_eq!(chip.reg[0xA], 0xB);
        assert_eq!(chip.reg[0x2], 0xC);
        assert_eq!(chip.pc, 512);
        chip.emulate_cycle();
        assert_eq!(chip.reg[0xA], 0xB ^ 0xC);
        assert_eq!(chip.reg[0x2], 0xC);
        assert_eq!(chip.pc, 514);
    }

    #[test]
    fn op_8xy4() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x8A, 0xB4, 0x8B, 0xC4]);
        chip.reg[0xA] = 0x00;
        chip.reg[0xB] = 0xFF;
        chip.reg[0xC] = 0x01;
        assert_eq!(chip.pc, 512);

        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        assert_eq!(chip.reg[0xA], 0xFF);
        assert_eq!(chip.reg[0xB], 0xFF);
        assert_eq!(chip.reg[0xF], 0x00);

        chip.emulate_cycle();
        assert_eq!(chip.pc, 516);
        assert_eq!(chip.reg[0xB], 0x0);
        assert_eq!(chip.reg[0xC], 0x1);
        assert_eq!(chip.reg[0xF], 0x1);
    }

    #[test]
    fn op_8xy5() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x8A, 0xB5, 0x8A, 0xB5]);
        chip.reg[0xA] = 0x01;
        chip.reg[0xB] = 0x02;
        assert_eq!(chip.pc, 512);

        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        assert_eq!(chip.reg[0xA], 0xFF);
        assert_eq!(chip.reg[0xB], 0x02);
        assert_eq!(chip.reg[0xF], 0x00);
        chip.reg[0xA] = 0x02;
        chip.reg[0xB] = 0x01;

        chip.emulate_cycle();
        assert_eq!(chip.pc, 516);
        assert_eq!(chip.reg[0xA], 0x1);
        assert_eq!(chip.reg[0xB], 0x1);
        assert_eq!(chip.reg[0xF], 0x1);
    }

    #[test]
    fn op_8x06() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x81, 0x06]);
        chip.reg[0x1] = 0b011;
        assert_eq!(chip.pc, 512);
        assert_eq!(chip.reg[0xF], 0);
        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        assert_eq!(chip.reg[0x1], 0b01);
        assert_eq!(chip.reg[0xF], 0x1);
    }

    #[test]
    fn op_8xy7() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x8A, 0xB7, 0x8A, 0xB7]);
        chip.reg[0xA] = 0x01;
        chip.reg[0xB] = 0x02;
        assert_eq!(chip.pc, 512);

        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        assert_eq!(chip.reg[0xA], 0x01);
        assert_eq!(chip.reg[0xB], 0x02);
        assert_eq!(chip.reg[0xF], 0x01);
        chip.reg[0xA] = 0x02;
        chip.reg[0xB] = 0x01;

        chip.emulate_cycle();
        assert_eq!(chip.pc, 516);
        assert_eq!(chip.reg[0xA], 0xFF);
        assert_eq!(chip.reg[0xB], 0x01);
        assert_eq!(chip.reg[0xF], 0x0);
    }

    #[test]
    fn op_8x0e() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x81, 0x0E]);
        chip.reg[0x1] = 0x81;
        assert_eq!(chip.pc, 512);
        assert_eq!(chip.reg[0xF], 0);
        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        assert_eq!(chip.reg[0x1], 0x81 << 1);
        assert_eq!(chip.reg[0xF], 0x1);
    }

    #[test]
    fn op_9xy0() {
        let mut chip = Chip8::default();
        chip.load(&vec![0x91, 0x20, 0x91, 0x30]);
        chip.reg[0x1] = 0x81;
        chip.reg[0x2] = 0x81;
        chip.reg[0x3] = 0x82;
        assert_eq!(chip.pc, 512);

        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);

        chip.emulate_cycle();
        assert_eq!(chip.pc, 518);
    }

    #[test]
    fn op_annn() {
        let mut chip = Chip8::default();
        chip.load(&vec![0xA6, 0x66]);
        assert_eq!(chip.index, 0);
        assert_eq!(chip.pc, 512);
        chip.emulate_cycle();
        assert_eq!(chip.index, 0x666);
        assert_eq!(chip.pc, 514);
    }

    #[test]
    fn op_bnnn() {
        let mut chip = Chip8::default();
        chip.load(&vec![0xB6, 0x66]);
        chip.reg[0] = 0x5;
        assert_eq!(chip.index, 0);
        assert_eq!(chip.pc, 512);
        chip.emulate_cycle();
        assert_eq!(chip.index, 0);
        assert_eq!(chip.pc, 0x666 + 0x5);
    }

    #[test]
    fn op_ex9e() {
        let mut chip = Chip8::default();
        chip.load(&vec![0xE1, 0x9E, 0xE1, 0x9E]);
        chip.reg[1] = 1;
        chip.key[1] = 0;
        assert_eq!(chip.pc, 512);

        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        chip.key[1] = 1;

        chip.emulate_cycle();
        assert_eq!(chip.pc, 518);
    }

    #[test]
    fn op_exa1() {
        let mut chip = Chip8::default();
        chip.load(&vec![0xE1, 0xA1, 0xE1, 0xA1]);
        chip.reg[1] = 1;
        chip.key[1] = 1;
        assert_eq!(chip.pc, 512);

        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        chip.key[1] = 0;

        chip.emulate_cycle();
        assert_eq!(chip.pc, 518);
    }

    #[test]
    fn op_fx07() {
        let mut chip = Chip8::default();
        chip.load(&vec![0xF1, 0x07]);
        chip.timer_delay = 10;
        assert_eq!(chip.pc, 512);

        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        assert_eq!(chip.reg[1], 10);
    }

    #[test]
    fn op_fx0a() {
        let mut chip = Chip8::default();
        chip.load(&vec![0xF1, 0x0A]);
        assert_eq!(chip.reg[1], 0);
        assert_eq!(chip.key[1], 0);
        assert_eq!(chip.pc, 512);

        chip.emulate_cycle();
        assert_eq!(chip.pc, 512);
        chip.key[1] = 1;

        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        assert_eq!(chip.reg[1], 1);
    }

    #[test]
    fn op_fx15() {
        let mut chip = Chip8::default();
        chip.load(&vec![0xF1, 0x15]);
        chip.reg[1] = 10;
        assert_eq!(chip.pc, 512);

        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        assert_eq!(chip.timer_delay, 9);
    }

    #[test]
    fn op_fx18() {
        let mut chip = Chip8::default();
        chip.load(&vec![0xF1, 0x18]);
        chip.reg[1] = 10;
        assert_eq!(chip.pc, 512);

        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        assert_eq!(chip.timer_sound, 9);
    }

    #[test]
    fn op_fx1e() {
        let mut chip = Chip8::default();
        chip.load(&vec![0xF1, 0x1E]);
        chip.reg[1] = 10;
        let init_index = chip.index;
        assert_eq!(chip.pc, 512);

        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        assert_eq!(chip.index, init_index + chip.reg[1] as u16);
        assert_eq!(chip.reg[1], 10);
    }

    #[test]
    fn op_fx29() {
        let mut chip = Chip8::default();
        chip.load(&vec![0xF1, 0x29]);
        chip.reg[1] = 0xA;
        assert_eq!(chip.pc, 512);

        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        assert_eq!(chip.index, chip.reg[1] as u16 * 5);
        assert_eq!(chip.reg[1], 0xA);
    }

    #[test]
    fn op_fx55() {
        let mut chip = Chip8::default();
        chip.load(&vec![0xF1, 0x55]);
        chip.index = 10;
        chip.reg[0] = 0xAB;
        chip.reg[1] = 0xCD;
        assert_eq!(chip.pc, 512);

        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        assert_eq!(chip.memory[10], 0xAB);
        assert_eq!(chip.memory[11], 0xCD);
    }

    #[test]
    fn op_fx65() {
        let mut chip = Chip8::default();
        chip.load(&vec![0xF1, 0x65]);
        chip.memory[10] = 0xAB;
        chip.memory[11] = 0xCD;
        chip.index = 10;
        assert_eq!(chip.pc, 512);

        chip.emulate_cycle();
        assert_eq!(chip.pc, 514);
        assert_eq!(chip.reg[0], 0xAB);
        assert_eq!(chip.reg[1], 0xCD);
    }
}
