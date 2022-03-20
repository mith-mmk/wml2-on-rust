
#[derive(Debug)]
pub struct RGB {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Debug)]
pub struct RGBA {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

/*
const COLOR2: [RGB;2] = [
    RGB{red:0x00,green:0x00,blue:0x00},    // 0 black
    RGB{red:0xff,green:0xff,blue:0xff},    // 1 white
];

const VGACOLOR256: [RGB;256] = [
    // System color 0-15
    RGB{red:0x00, green:  0x00, blue: 0x00},    //  0 black
	RGB{red:0x00, green:  0x00, blue: 0xaa},    //  1 dark blue
	RGB{red:0x00, green:  0xaa, blue: 0x00},    //  2 dark green
	RGB{red:0x00, green:  0xaa, blue: 0xaa},    //  3 dark cyan
	RGB{red:0xaa, green:  0x00, blue: 0x00},    //  4 dark red
	RGB{red:0xaa, green:  0x00, blue: 0xaa},    //  5 dark magenta
	RGB{red:0xaa, green:  0x55, blue: 0x00},    //  6 dark yellow
	RGB{red:0xaa, green:  0xaa, blue: 0xaa},    //  7 dark white
	RGB{red:0x55, green:  0x55, blue: 0x55},    //  8 light gray
	RGB{red:0x55, green:  0x55, blue: 0xff},    //  9 light blue
	RGB{red:0x55, green:  0xff, blue: 0x55},    // 10 light green
	RGB{red:0x55, green:  0xff, blue: 0xff},    // 11 light cyan
	RGB{red:0xff, green:  0x55, blue: 0x55},    // 12 light red
	RGB{red:0xff, green:  0x55, blue: 0xff},    // 13 light magenta
	RGB{red:0xff, green:  0xff, blue: 0x55},    // 14 light yelow
	RGB{red:0xff, green:  0xff, blue: 0xff},    // 15 white
    // grayscale 16-31
	RGB{red:0x00, green:  0x00, blue: 0x00},    // 16
	RGB{red:0x14, green:  0x14, blue: 0x14},    // 17
	RGB{red:0x20, green:  0x20, blue: 0x20},    // 18
	RGB{red:0x2c, green:  0x2c, blue: 0x2c},    // 19
	RGB{red:0x38, green:  0x38, blue: 0x38},    // 20
	RGB{red:0x45, green:  0x45, blue: 0x45},    // 21
	RGB{red:0x51, green:  0x51, blue: 0x51},    // 22
	RGB{red:0x61, green:  0x61, blue: 0x61},    // 23
	RGB{red:0x71, green:  0x71, blue: 0x71},    // 24
	RGB{red:0x82, green:  0x82, blue: 0x82},    // 25
	RGB{red:0x92, green:  0x92, blue: 0x92},    // 26
	RGB{red:0xa2, green:  0xa2, blue: 0xa2},    // 27
	RGB{red:0xb6, green:  0xb6, blue: 0xb6},    // 28
	RGB{red:0xcb, green:  0xcb, blue: 0xcb},    // 29
	RGB{red:0xe3, green:  0xe3, blue: 0xe3},    // 30
	RGB{red:0xff, green:  0xff, blue: 0xff},    // 31
    // BLUE -> GREEN
	RGB{red:0x00, green:  0x00, blue: 0xff},
	RGB{red:0x41, green:  0x00, blue: 0xff},
	RGB{red:0x7d, green:  0x00, blue: 0xff},
	RGB{red:0xbe, green:  0x00, blue: 0xff},
	RGB{red:0xff, green:  0x00, blue: 0xff},
	RGB{red:0xff, green:  0x00, blue: 0xbe},
	RGB{red:0xff, green:  0x00, blue: 0x7d},
	RGB{red:0xff, green:  0x00, blue: 0x41},
	RGB{red:0xff, green:  0x00, blue: 0x00},
	RGB{red:0xff, green:  0x41, blue: 0x00},
	RGB{red:0xff, green:  0x7d, blue: 0x00},
	RGB{red:0xff, green:  0xbe, blue: 0x00},
	RGB{red:0xff, green:  0xff, blue: 0x00},
	RGB{red:0xbe, green:  0xff, blue: 0x00},
	RGB{red:0x7d, green:  0xff, blue: 0x00},
	RGB{red:0x41, green:  0xff, blue: 0x00},
	RGB{red:0x00, green:  0xff, blue: 0x00},
    // light colors
	RGB{red:0x00, green:  0xff, blue: 0x41},
	RGB{red:0x00, green:  0xff, blue: 0x7d},
	RGB{red:0x00, green:  0xff, blue: 0xbe},
	RGB{red:0x00, green:  0xff, blue: 0xff},
	RGB{red:0x00, green:  0xbe, blue: 0xff},
	RGB{red:0x00, green:  0x7d, blue: 0xff},
	RGB{red:0x00, green:  0x41, blue: 0xff},
	RGB{red:0x7d, green:  0x7d, blue: 0xff},
	RGB{red:0x9e, green:  0x7d, blue: 0xff},
	RGB{red:0xbe, green:  0x7d, blue: 0xff},
	RGB{red:0xdf, green:  0x7d, blue: 0xff},
	RGB{red:0xff, green:  0x7d, blue: 0xff},
	RGB{red:0xff, green:  0x7d, blue: 0xdf},
	RGB{red:0xff, green:  0x7d, blue: 0xbe},
	RGB{red:0xff, green:  0x7d, blue: 0x9e},
	RGB{red:0xff, green:  0x7d, blue: 0x7d},
	RGB{red:0xff, green:  0x9e, blue: 0x7d},
	RGB{red:0xff, green:  0xbe, blue: 0x7d},
	RGB{red:0xff, green:  0xdf, blue: 0x7d},
	RGB{red:0xff, green:  0xff, blue: 0x7d},
	RGB{red:0xdf, green:  0xff, blue: 0x7d},
	RGB{red:0xbe, green:  0xff, blue: 0x7d},
	RGB{red:0x9e, green:  0xff, blue: 0x7d},
	RGB{red:0x7d, green:  0xff, blue: 0x7d},
	RGB{red:0x7d, green:  0xff, blue: 0x9e},
	RGB{red:0x7d, green:  0xff, blue: 0xbe},
	RGB{red:0x7d, green:  0xff, blue: 0xdf},
	RGB{red:0x7d, green:  0xff, blue: 0xff},
	RGB{red:0x7d, green:  0xdf, blue: 0xff},
	RGB{red:0x7d, green:  0xbe, blue: 0xff},
	RGB{red:0x7d, green:  0x9e, blue: 0xff},
	RGB{red:0xb6, green:  0xb6, blue: 0xff},
	RGB{red:0xc7, green:  0xb6, blue: 0xff},
	RGB{red:0xdb, green:  0xb6, blue: 0xff},
	RGB{red:0xeb, green:  0xb6, blue: 0xff},
	RGB{red:0xff, green:  0xb6, blue: 0xff},
	RGB{red:0xff, green:  0xb6, blue: 0xeb},
	RGB{red:0xff, green:  0xb6, blue: 0xdb},
	RGB{red:0xff, green:  0xb6, blue: 0xc7},
	RGB{red:0xff, green:  0xb6, blue: 0xb6},
	RGB{red:0xff, green:  0xc7, blue: 0xb6},
	RGB{red:0xff, green:  0xdb, blue: 0xb6},
	RGB{red:0xff, green:  0xeb, blue: 0xb6},
	RGB{red:0xff, green:  0xff, blue: 0xb6},
	RGB{red:0xeb, green:  0xff, blue: 0xb6},
	RGB{red:0xdb, green:  0xff, blue: 0xb6},
	RGB{red:0xc7, green:  0xff, blue: 0xb6},
	RGB{red:0xb6, green:  0xff, blue: 0xb6},
	RGB{red:0xb6, green:  0xff, blue: 0xc7},
	RGB{red:0xb6, green:  0xff, blue: 0xdb},
	RGB{red:0xb6, green:  0xff, blue: 0xeb},
	RGB{red:0xb6, green:  0xff, blue: 0xff},
	RGB{red:0xb6, green:  0xeb, blue: 0xff},
	RGB{red:0xb6, green:  0xdb, blue: 0xff},
	RGB{red:0xb6, green:  0xc7, blue: 0xff},
    // Dark colors
	RGB{red:0x00, green:  0x00, blue: 0x71},
	RGB{red:0x1c, green:  0x00, blue: 0x71},
	RGB{red:0x38, green:  0x00, blue: 0x71},
	RGB{red:0x55, green:  0x00, blue: 0x71},
	RGB{red:0x71, green:  0x00, blue: 0x71},
	RGB{red:0x71, green:  0x00, blue: 0x55},
	RGB{red:0x71, green:  0x00, blue: 0x38},
	RGB{red:0x71, green:  0x00, blue: 0x1c},
	RGB{red:0x71, green:  0x00, blue: 0x00},
	RGB{red:0x71, green:  0x1c, blue: 0x00},
	RGB{red:0x71, green:  0x38, blue: 0x00},
	RGB{red:0x71, green:  0x55, blue: 0x00},
	RGB{red:0x71, green:  0x71, blue: 0x00},
	RGB{red:0x55, green:  0x71, blue: 0x00},
	RGB{red:0x38, green:  0x71, blue: 0x00},
	RGB{red:0x1c, green:  0x71, blue: 0x00},
	RGB{red:0x00, green:  0x71, blue: 0x00},
	RGB{red:0x00, green:  0x71, blue: 0x1c},
	RGB{red:0x00, green:  0x71, blue: 0x38},
	RGB{red:0x00, green:  0x71, blue: 0x55},
	RGB{red:0x00, green:  0x71, blue: 0x71},
	RGB{red:0x00, green:  0x55, blue: 0x71},
	RGB{red:0x00, green:  0x38, blue: 0x71},
	RGB{red:0x00, green:  0x1c, blue: 0x71},
	RGB{red:0x38, green:  0x38, blue: 0x71},
	RGB{red:0x45, green:  0x38, blue: 0x71},
	RGB{red:0x55, green:  0x38, blue: 0x71},
	RGB{red:0x61, green:  0x38, blue: 0x71},
	RGB{red:0x71, green:  0x38, blue: 0x71},
	RGB{red:0x71, green:  0x38, blue: 0x61},
	RGB{red:0x71, green:  0x38, blue: 0x55},
	RGB{red:0x71, green:  0x38, blue: 0x45},
	RGB{red:0x71, green:  0x38, blue: 0x38},
	RGB{red:0x71, green:  0x45, blue: 0x38},
	RGB{red:0x71, green:  0x55, blue: 0x38},
	RGB{red:0x71, green:  0x61, blue: 0x38},
	RGB{red:0x71, green:  0x71, blue: 0x38},
	RGB{red:0x61, green:  0x71, blue: 0x38},
	RGB{red:0x55, green:  0x71, blue: 0x38},
	RGB{red:0x45, green:  0x71, blue: 0x38},
	RGB{red:0x38, green:  0x71, blue: 0x38},
	RGB{red:0x38, green:  0x71, blue: 0x45},
	RGB{red:0x38, green:  0x71, blue: 0x55},
	RGB{red:0x38, green:  0x71, blue: 0x61},
	RGB{red:0x38, green:  0x71, blue: 0x71},
	RGB{red:0x38, green:  0x61, blue: 0x71},
	RGB{red:0x38, green:  0x55, blue: 0x71},
	RGB{red:0x38, green:  0x45, blue: 0x71},
	RGB{red:0x51, green:  0x51, blue: 0x71},
	RGB{red:0x59, green:  0x51, blue: 0x71},
	RGB{red:0x61, green:  0x51, blue: 0x71},
	RGB{red:0x69, green:  0x51, blue: 0x71},
	RGB{red:0x71, green:  0x51, blue: 0x71},
	RGB{red:0x71, green:  0x51, blue: 0x69},
	RGB{red:0x71, green:  0x51, blue: 0x61},
	RGB{red:0x71, green:  0x51, blue: 0x59},
	RGB{red:0x71, green:  0x51, blue: 0x51},
	RGB{red:0x71, green:  0x59, blue: 0x51},
	RGB{red:0x71, green:  0x61, blue: 0x51},
	RGB{red:0x71, green:  0x69, blue: 0x51},
	RGB{red:0x71, green:  0x71, blue: 0x51},
	RGB{red:0x69, green:  0x71, blue: 0x51},
	RGB{red:0x61, green:  0x71, blue: 0x51},
	RGB{red:0x59, green:  0x71, blue: 0x51},
	RGB{red:0x51, green:  0x71, blue: 0x51},
	RGB{red:0x51, green:  0x71, blue: 0x59},
	RGB{red:0x51, green:  0x71, blue: 0x61},
	RGB{red:0x51, green:  0x71, blue: 0x69},
	RGB{red:0x51, green:  0x71, blue: 0x71},
	RGB{red:0x51, green:  0x69, blue: 0x71},
	RGB{red:0x51, green:  0x61, blue: 0x71},
	RGB{red:0x51, green:  0x59, blue: 0x71},
	RGB{red:0x00, green:  0x00, blue: 0x41},
	RGB{red:0x10, green:  0x00, blue: 0x41},
	RGB{red:0x20, green:  0x00, blue: 0x41},
	RGB{red:0x30, green:  0x00, blue: 0x41},
	RGB{red:0x41, green:  0x00, blue: 0x41},
	RGB{red:0x41, green:  0x00, blue: 0x30},
	RGB{red:0x41, green:  0x00, blue: 0x20},
	RGB{red:0x41, green:  0x00, blue: 0x10},
	RGB{red:0x41, green:  0x00, blue: 0x00},
	RGB{red:0x41, green:  0x10, blue: 0x00},
	RGB{red:0x41, green:  0x20, blue: 0x00},
	RGB{red:0x41, green:  0x30, blue: 0x00},
	RGB{red:0x41, green:  0x41, blue: 0x00},
	RGB{red:0x30, green:  0x41, blue: 0x00},
	RGB{red:0x20, green:  0x41, blue: 0x00},
	RGB{red:0x10, green:  0x41, blue: 0x00},
	RGB{red:0x00, green:  0x41, blue: 0x00},
	RGB{red:0x00, green:  0x41, blue: 0x10},
	RGB{red:0x00, green:  0x41, blue: 0x20},
	RGB{red:0x00, green:  0x41, blue: 0x30},
	RGB{red:0x00, green:  0x41, blue: 0x41},
	RGB{red:0x00, green:  0x30, blue: 0x41},
	RGB{red:0x00, green:  0x20, blue: 0x41},
	RGB{red:0x00, green:  0x10, blue: 0x41},
	RGB{red:0x20, green:  0x20, blue: 0x41},
	RGB{red:0x28, green:  0x20, blue: 0x41},
	RGB{red:0x30, green:  0x20, blue: 0x41},
	RGB{red:0x38, green:  0x20, blue: 0x41},
	RGB{red:0x41, green:  0x20, blue: 0x41},
	RGB{red:0x41, green:  0x20, blue: 0x38},
	RGB{red:0x41, green:  0x20, blue: 0x30},
	RGB{red:0x41, green:  0x20, blue: 0x28},
	RGB{red:0x41, green:  0x20, blue: 0x20},
	RGB{red:0x41, green:  0x28, blue: 0x20},
	RGB{red:0x41, green:  0x30, blue: 0x20},
	RGB{red:0x41, green:  0x38, blue: 0x20},
	RGB{red:0x41, green:  0x41, blue: 0x20},
	RGB{red:0x38, green:  0x41, blue: 0x20},
	RGB{red:0x30, green:  0x41, blue: 0x20},
	RGB{red:0x28, green:  0x41, blue: 0x20},
	RGB{red:0x20, green:  0x41, blue: 0x20},
	RGB{red:0x20, green:  0x41, blue: 0x28},
	RGB{red:0x20, green:  0x41, blue: 0x30},
	RGB{red:0x20, green:  0x41, blue: 0x38},
	RGB{red:0x20, green:  0x41, blue: 0x41},
	RGB{red:0x20, green:  0x38, blue: 0x41},
	RGB{red:0x20, green:  0x30, blue: 0x41},
	RGB{red:0x20, green:  0x28, blue: 0x41},
	RGB{red:0x2c, green:  0x2c, blue: 0x41},
	RGB{red:0x30, green:  0x2c, blue: 0x41},
	RGB{red:0x34, green:  0x2c, blue: 0x41},
	RGB{red:0x3c, green:  0x2c, blue: 0x41},
	RGB{red:0x41, green:  0x2c, blue: 0x41},
	RGB{red:0x41, green:  0x2c, blue: 0x3c},
	RGB{red:0x41, green:  0x2c, blue: 0x34},
	RGB{red:0x41, green:  0x2c, blue: 0x30},
	RGB{red:0x41, green:  0x2c, blue: 0x2c},
	RGB{red:0x41, green:  0x30, blue: 0x2c},
	RGB{red:0x41, green:  0x34, blue: 0x2c},
	RGB{red:0x41, green:  0x3c, blue: 0x2c},
	RGB{red:0x41, green:  0x41, blue: 0x2c},
	RGB{red:0x3c, green:  0x41, blue: 0x2c},
	RGB{red:0x34, green:  0x41, blue: 0x2c},
	RGB{red:0x30, green:  0x41, blue: 0x2c},
	RGB{red:0x2c, green:  0x41, blue: 0x2c},
	RGB{red:0x2c, green:  0x41, blue: 0x30},
	RGB{red:0x2c, green:  0x41, blue: 0x34},
	RGB{red:0x2c, green:  0x41, blue: 0x3c},
	RGB{red:0x2c, green:  0x41, blue: 0x41},
	RGB{red:0x2c, green:  0x3c, blue: 0x41},
	RGB{red:0x2c, green:  0x34, blue: 0x41},
	RGB{red:0x2c, green:  0x30, blue: 0x41},
    // black
	RGB{red:0x00, green:  0x00, blue: 0x00},
	RGB{red:0x00, green:  0x00, blue: 0x00},
	RGB{red:0x00, green:  0x00, blue: 0x00},
	RGB{red:0x00, green:  0x00, blue: 0x00},
	RGB{red:0x00, green:  0x00, blue: 0x00},
	RGB{red:0x00, green:  0x00, blue: 0x00},
	RGB{red:0x00, green:  0x00, blue: 0x00},
	RGB{red:0x00, green:  0x00, blue: 0x00},
];

const PC98COLOR16: [RGB; 16] = [
    RGB{red:0x00,green:0x00,blue:0x00},    //  0 black
    RGB{red:0x00,green:0x00,blue:0x7f},    //  1 dark blue
    RGB{red:0x7f,green:0x00,blue:0xff},    //  2 dark red
    RGB{red:0x7f,green:0x00,blue:0x7f},    //  3 dark magenta
    RGB{red:0x00,green:0x7f,blue:0x00},    //  4 dark green
    RGB{red:0x00,green:0x7f,blue:0x7f},    //  5 dark cyan
    RGB{red:0x80,green:0x7f,blue:0x00},    //  6 dark yellow
    RGB{red:0x7f,green:0x7f,blue:0x7f},    //  7 dark white
    RGB{red:0x4f,green:0x4f,blue:0x4f},    //  8 light black
    RGB{red:0x00,green:0x00,blue:0xff},    //  9 blue
    RGB{red:0xff,green:0x00,blue:0x00},    // 10 red
    RGB{red:0xff,green:0x00,blue:0xff},    // 11 magenta
    RGB{red:0x00,green:0xff,blue:0x00},    // 12 green
    RGB{red:0x00,green:0xff,blue:0xff},    // 13 cyan
    RGB{red:0xff,green:0xff,blue:0x00},    // 14 yellow
    RGB{red:0xff,green:0xff,blue:0xff},    // 15 white
];
*/