pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;
const RAM_SIZE: usize = 0x1000;
const NUM_REGS: usize = 16;
const STACK_SIZE: usize = 16;
const NUM_KEYS: usize = 16;
/// The first `0x200` bytes were used to run Chip-8 itself.
const START_ADDR: u16 = 0x200;

pub struct Emu {
    /// Program counter
    pc: u16,
    ram: [u8; RAM_SIZE],
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    v_reg: [u8; NUM_REGS],
    /// I register, which is used for indexing into RAM for reads and writes.
    i_reg: u16,
    /// Stack pointer
    sp: u16,
    stack: [u16; STACK_SIZE],
    keys: [bool; NUM_KEYS],
    /// Delay Timer, which performs some action when it hits `0`.
    dt: u8,
    /// Sound Timer, which emits audio when it hits `0`.
    st: u8,
}

impl Emu {
    pub fn new() -> Self {
        let mut new_emu = Self {
            pc: START_ADDR,
            ram: [0; RAM_SIZE],
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            v_reg: [0; NUM_REGS],
            i_reg: 0,
            sp: 0,
            stack: [0; STACK_SIZE],
            keys: [false; NUM_KEYS],
            dt: 0,
            st: 0,
        };
        // Load number fonts
        new_emu.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
        new_emu
    }

    pub fn tick(&mut self) {
        let op = self.fetch();
        self.execute(op);
    }

    pub fn get_display(&self) -> &[bool] {
        &self.screen
    }

    pub fn keypress(&mut self, idx: usize, pressed: bool) {
        self.keys[idx] = pressed;
    }

    pub fn load(&mut self, data: &[u8]) {
        let start = START_ADDR as usize;
        let end = start + data.len();
        self.ram[start..end].copy_from_slice(data);
    }

    fn execute(&mut self, op: u16) {
        let d1 = (op & 0xF000) >> 12;
        let d2 = (op & 0x0F00) >> 8;
        let d3 = (op & 0x00F0) >> 4;
        let d4 = op & 0x000F;

        match (d1, d2, d3, d4) {
            (1, _, _, _) => {
                let nnn = op & 0xFFF;
                self.pc = nnn;
            }
            (2, _, _, _) => {
                let nnn = op & 0xFFF;
                self.push(self.pc);
                self.pc = nnn;
            }
            (3, _, _, _) => {
                let x = d2 as usize;
                let nn = (op & 0xFF) as u8;
                if self.v_reg[x] == nn {
                    self.step();
                }
            }
            (4, _, _, _) => {
                let x = d2 as usize;
                let nn = (op & 0xFF) as u8;
                if self.v_reg[x] != nn {
                    self.step();
                }
            }
            (5, _, _, 0) => {
                let x = d2 as usize;
                let y = d3 as usize;
                if self.v_reg[x] == self.v_reg[y] {
                    self.pc += 2;
                }
            }
            (6, _, _, _) => {
                let x = d2 as usize;
                let nn = (op & 0xFF) as u8;
                self.v_reg[x] = nn;
            }
            (7, _, _, _) => {
                let x = d2 as usize;
                let nn = (op & 0xFF) as u8;
                self.v_reg[x] = self.v_reg[x].wrapping_add(nn);
            }
            (8, _, _, 0) => {
                let x = d2 as usize;
                let y = d3 as usize;
                self.v_reg[x] = self.v_reg[y];
            }
            (8, _, _, 1) => {
                let x = d2 as usize;
                let y = d3 as usize;
                self.v_reg[x] |= self.v_reg[y];
            }
            (8, _, _, 2) => {
                let x = d2 as usize;
                let y = d3 as usize;
                self.v_reg[x] &= self.v_reg[y];
            }
            (8, _, _, 3) => {
                let x = d2 as usize;
                let y = d3 as usize;
                self.v_reg[x] ^= self.v_reg[y];
            }
            (8, _, _, 4) => {
                let x = d2 as usize;
                let y = d3 as usize;
                let (val, overflow) = self.v_reg[x].overflowing_add(self.v_reg[y]);
                self.v_reg[x] = val;
                self.v_reg[0xF] = if overflow { 1 } else { 0 };
            }
            (8, _, _, 5) => {
                let x = d2 as usize;
                let y = d3 as usize;
                let (val, overflow) = self.v_reg[x].overflowing_sub(self.v_reg[y]);
                self.v_reg[x] = val;
                self.v_reg[0xF] = if overflow { 1 } else { 0 };
            }
            (8, _, _, 6) => {
                let x = d2 as usize;
                let lsb = self.v_reg[x] & 1;
                self.v_reg[0xF] = lsb;
                self.v_reg[x] >>= 1;
            }
            (8, _, _, 7) => {
                let x = d2 as usize;
                let y = d3 as usize;
                let (val, overflow) = self.v_reg[y].overflowing_sub(self.v_reg[x]);
                self.v_reg[x] = val;
                self.v_reg[0xF] = if overflow { 1 } else { 0 };
            }
            (8, _, _, 0xE) => {
                let x = d2 as usize;
                let msb = self.v_reg[x] >> 7 & 1;
                self.v_reg[0xF] = msb;
                self.v_reg[x] <<= 1;
            }
            (9, _, _, 0) => {
                let x = d2 as usize;
                let y = d3 as usize;
                if self.v_reg[x] != self.v_reg[y] {
                    self.pc += 2;
                }
            }
            (0xA, _, _, _) => {
                let nnn = op & 0xFFF;
                self.i_reg = nnn;
            }
            (0xB, _, _, _) => {
                let nnn = op & 0xFFF;
                self.pc = nnn + self.v_reg[0] as u16;
            }
            (0xC, _, _, _) => {
                let x = d2 as usize;
                let nn = (op & 0xFF) as u8;
                self.v_reg[x] = rand::random::<u8>() & nn;
            }
            (0xD, _, _, _) => {
                let x = d2 as usize;
                let y = d3 as usize;
                let x_coor = self.v_reg[x] as usize;
                let y_coor = self.v_reg[y] as usize;
                let num_rows = d4 as usize;
                let mut flipped = false;
                for y_line in 0..num_rows {
                    let addr = self.i_reg + y_line as u16;
                    let pixels = self.ram[addr as usize];
                    for x_line in 0..8 {
                        if (pixels & (0b1000_0000 >> x_line)) != 0 {
                            let x = (x_coor + x_line) % SCREEN_WIDTH;
                            let y = (y_coor + y_line) % SCREEN_HEIGHT;

                            let idx = x + SCREEN_WIDTH * y;
                            flipped |= self.screen[idx];
                            self.screen[idx] ^= true;
                        }
                    }
                }

                self.v_reg[0xF] = if flipped { 1 } else { 0 };
            }
            (0xE, _, 9, 0xE) => {
                let x = d2 as usize;
                let vx = self.v_reg[x];
                let key = self.keys[vx as usize];
                if key {
                    self.step();
                }
            }
            (0xE, _, 0xA, 1) => {
                let x = d2 as usize;
                let vx = self.v_reg[x];
                let key = self.keys[vx as usize];
                if !key {
                    self.step();
                }
            }
            (0xF, _, 0, 7) => {
                let x = d2 as usize;
                self.v_reg[x] = self.dt;
            }
            (0xF, _, 0, 0xA) => {
                let x = d2 as usize;
                let mut pressed = false;
                for (i, key) in self.keys.iter().enumerate() {
                    if *key {
                        self.v_reg[x] = i as u8;
                        pressed = true;
                        break;
                    }
                }
                if !pressed {
                    self.step_backward();
                }
            }
            (0xF, _, 1, 5) => {
                let x = d2 as usize;
                self.dt = self.v_reg[x];
            }
            (0xF, _, 1, 8) => {
                let x = d2 as usize;
                self.st = self.v_reg[x];
            }
            (0xF, _, 1, 0xE) => {
                let x = d2 as usize;
                let vx = self.v_reg[x] as u16;
                self.i_reg = self.i_reg.wrapping_add(vx);
            }
            (0xF, _, 2, 9) => {
                let x = d2 as usize;
                let c = self.v_reg[x] as u16;
                self.i_reg = c * 5;
            }
            (0xF, _, 3, 3) => {
                let x = d2 as usize;
                let vx = self.v_reg[x];
                let hundreds = vx / 100;
                let tens = (vx / 10) % 10;
                let ones = vx % 10;
                let i_reg = self.i_reg as usize;
                self.ram[i_reg] = hundreds;
                self.ram[i_reg + 1] = tens;
                self.ram[i_reg + 2] = ones;
            }
            (0xF, _, 5, 5) => {
                let x = d2 as usize;
                let i = self.i_reg as usize;
                for idx in 0..=x {
                    self.ram[i + idx] = self.v_reg[idx];
                }
            }
            (0xF, _, 6, 5) => {
                let x = d2 as usize;
                let i = self.i_reg as usize;
                for idx in 0..=x {
                    self.v_reg[idx] = self.ram[i + idx];
                }
            }
            (0, 0, 0xE, 0xE) => {
                let pc = self.pop();
                self.pc = pc;
            }
            (0, 0, 0xE, 0) => self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            (0, 0, 0, 0) => {}
            _ => panic!("Invalid opcode"),
        }
    }

    fn step_backward(&mut self) {
        self.pc -= 2;
    }

    fn step(&mut self) {
        self.pc += 2;
    }

    /// Returns the current opcode in the form of u16
    /// and increases the program counter.
    fn fetch(&mut self) -> u16 {
        let high_byte = self.ram[self.pc as usize] as u16;
        let low_byte = self.ram[(self.pc + 1) as usize] as u16;

        self.step();

        high_byte << 8 | low_byte
    }

    pub fn tick_timers(&mut self) {
        if self.dt > 0 {
            self.dt -= 1;
        }
        if self.st > 0 {
            if self.st == 1 {
                self.beep();
            }
            self.st -= 1;
        }
    }

    fn beep(&self) {
        // TODO: implement beeping
    }

    pub fn reset(&mut self) {
        self.pc = START_ADDR;
        self.ram = [0; RAM_SIZE];
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        self.v_reg = [0; NUM_REGS];
        self.i_reg = 0;
        self.sp = 0;
        self.stack = [0; STACK_SIZE];
        self.keys = [false; NUM_KEYS];
        self.dt = 0;
        self.st = 0;
        self.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
    }

    pub fn push(&mut self, val: u16) {
        self.stack[self.sp as usize] = val;
        self.sp += 1;
    }
    pub fn pop(&mut self) -> u16 {
        self.sp -= 1;
        self.stack[self.sp as usize]
    }
}

impl Default for Emu {
    fn default() -> Self {
        Self::new()
    }
}

const FONTSET_SIZE: usize = 80;
const FONTSET: [u8; FONTSET_SIZE] = [
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
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];
