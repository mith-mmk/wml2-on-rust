use crate::decoder::ccitt::HuffmanTree;

pub fn white_tree() -> HuffmanTree {
    let working_bits = 9;
    let max_bits = 12;
    let append_bits = 9;

    let append = [
        //EOL  	0000_0000_0001
        (-1, -1),
        (12, -2),
        (-1, -1),
        (-1, -1),
        (-1, -1),
        (-1, -1),
        (-1, -1),
        (-1, -1),
        (-1, -1),
        (-1, -1),
        (-1, -1),
        (-1, -1),
        (-1, -1),
        (-1, -1),
        (-1, -1),
        (-1, -1),
        // 1792    0000_0001_000
        (11, 1792),
        (11, 1792),
        // 1984    0000_0001_0010
        // 2048    0000_0001_0011
        (12, 1984),
        (12, 2048),
        // 2112    0000_0001_0100
        // 2176    0000_0001_0101
        // 2240    0000_0001_0110
        // 2304    0000_0001_0111
        (12, 2112),
        (12, 2176),
        (12, 2240),
        (12, 2304),
        // 1856    0000_0001_100
        (11, 1856),
        (11, 1856),
        // 1920    0000_0001_101
        (11, 1920),
        (11, 1920),
        // 2368    0000_0001_1100
        (12, 2368),
        // 2432    0000_0001_1101
        (12, 2432),
        // 2496    0000_0001_1110
        (12, 2496),
        // 2560    0000_0001_1111
        (12, 2560),
    ];

    let matrix: [(i32, i32); 512] = [
        (-1, 0),
        (-1, 1),
        (-1, 2),
        (-1, 3),
        // 29	    0000_0010
        (8, 29),
        (8, 29),
        // 30	    0000_0011
        (8, 30),
        (8, 30),
        // 45	    0000_0100
        (8, 45),
        (8, 45),
        // 46	    0000_0101
        (8, 46),
        (8, 46),
        // 22	    0000_011
        (7, 22),
        (7, 22),
        (7, 22),
        (7, 22),
        // 23	    0000_100

        // 23 0000_100
        (7, 23),
        (7, 23),
        (7, 23),
        (7, 23),
        // 47	    0000_1010
        // 48	    0000_1011
        (8, 47),
        (8, 47),
        (8, 48),
        (8, 48),
        // 13 0000_11
        (6, 13),
        (6, 13),
        (6, 13),
        (6, 13),
        (6, 13),
        (6, 13),
        (6, 13),
        (6, 13),
        // 20	    0001_000
        (7, 20),
        (7, 20),
        (7, 20),
        (7, 20),
        // 33	    0001_0010
        // 34	    0001_0011
        (8, 33),
        (8, 33),
        (8, 34),
        (8, 34),
        // 35	    0001_0100
        // 36	    0001_0101
        // 37	    0001_0110
        // 38	    0001_0111
        (8, 35),
        (8, 35),
        (8, 36),
        (8, 36),
        (8, 37),
        (8, 37),
        (8, 38),
        (8, 38),
        // 19	    0001_100
        (7, 19),
        (7, 19),
        (7, 19),
        (7, 19),
        // 31	    0001_1010
        // 32	    0001_1011
        (8, 31),
        (8, 31),
        (8, 32),
        (8, 32),
        //  1	    0001_11
        (6, 1),
        (6, 1),
        (6, 1),
        (6, 1),
        (6, 1),
        (6, 1),
        (6, 1),
        (6, 1),
        // 12	    0010_00
        (6, 12),
        (6, 12),
        (6, 12),
        (6, 12),
        (6, 12),
        (6, 12),
        (6, 12),
        (6, 12),
        // 53	    0010_0100
        // 54	    0010_0101
        (8, 53),
        (8, 53),
        (8, 54),
        (8, 54),
        // 26	    0010_011
        (7, 26),
        (7, 26),
        (7, 26),
        (7, 26),
        // 39	    0010_1000
        // 40	    0010_1001
        // 41	    0010_1010
        // 42	    0010_1011
        (8, 39),
        (8, 39),
        (8, 40),
        (8, 40),
        (8, 41),
        (8, 41),
        (8, 42),
        (8, 42),
        // 43  	    0010_1100
        // 44	    0010_1101
        (8, 43),
        (8, 43),
        (8, 44),
        (8, 44),
        // 21	    0010_111
        (7, 21),
        (7, 21),
        (7, 21),
        (7, 21),
        // 28	    0011_000
        (7, 28),
        (7, 28),
        (7, 28),
        (7, 28),
        // 61  	    0011_0010
        // 62	    0011_0011
        (8, 61),
        (8, 61),
        (8, 62),
        (8, 62),
        // 63	    0011_0100
        // 0	    0011_0101
        // 320	    0011_0110
        // 384	    0011_0111
        (8, 63),
        (8, 63),
        (8, 0),
        (8, 0),
        (8, 320),
        (8, 320),
        (8, 384),
        (8, 384),
        // 10	    0011_1
        (5, 10),
        (5, 10),
        (5, 10),
        (5, 10),
        (5, 10),
        (5, 10),
        (5, 10),
        (5, 10),
        (5, 10),
        (5, 10),
        (5, 10),
        (5, 10),
        (5, 10),
        (5, 10),
        (5, 10),
        (5, 10),
        // 01000
        // 11	    0100_0
        (5, 11),
        (5, 11),
        (5, 11),
        (5, 11),
        (5, 11),
        (5, 11),
        (5, 11),
        (5, 11),
        (5, 11),
        (5, 11),
        (5, 11),
        (5, 11),
        (5, 11),
        (5, 11),
        (5, 11),
        (5, 11),
        // 27	    0100_100
        (7, 27),
        (7, 27),
        (7, 27),
        (7, 27),
        // 0100_101
        // 59	    0100_1010
        // 60	    0100_1011
        (8, 59),
        (8, 59),
        (8, 60),
        (8, 60),
        // 0100_111

        // 1472	0100_1100_0
        // 1536	0100_1100_1
        (9, 1472),
        (9, 1536),
        // 1600	0100_1101_0
        // 1728	0100_1101_1
        (9, 1600),
        (9, 1728),
        // 18	    0100_111
        (7, 18),
        (7, 18),
        (7, 18),
        (7, 18),
        // 0101

        //  24	    0101_000
        (7, 24),
        (7, 24),
        (7, 24),
        (7, 24),
        // 49	    0101_0010
        // 50	    0101_0011
        (8, 49),
        (8, 49),
        (8, 50),
        (8, 50),
        // 51	    0101_0100
        // 52	    0101_0101
        (8, 51),
        (8, 51),
        (8, 52),
        (8, 52),
        // 25	    0101_011
        (7, 25),
        (7, 25),
        (7, 25),
        (7, 25),
        // 55	    0101_1000
        // 56	    0101_1001
        // 57	    0101_1010
        // 58	    0101_1011
        (8, 55),
        (8, 55),
        (8, 56),
        (8, 56),
        (8, 57),
        (8, 57),
        (8, 58),
        (8, 58),
        // 192 0101_10
        (6, 192),
        (6, 192),
        (6, 192),
        (6, 192),
        (6, 192),
        (6, 192),
        (6, 192),
        (6, 192),
        // 1664	0110_00
        (6, 1664),
        (6, 1664),
        (6, 1664),
        (6, 1664),
        (6, 1664),
        (6, 1664),
        (6, 1664),
        (6, 1664),
        // 448	    0110_0100
        // 512	    0110_0101
        (8, 448),
        (8, 448),
        (8, 512),
        (8, 512),
        // 704      0110_0110_0
        // 768	    0110_0110_1
        (9, 704),
        (9, 768),
        // 640      0110_0111
        (8, 640),
        (8, 640),
        // 576	    0110_1000
        (8, 576),
        (8, 576),
        // 832	    0110_1001_0
        // 896	    0110_1001_1
        (9, 832),
        (9, 896),
        // 960	0110_1010_0
        // 1024	0110_1010_1
        (9, 960),
        (9, 1024),
        // 1088	0110_1011_0
        // 1152	0110_1011_1
        (9, 1088),
        (9, 1152),
        // 1216	0110_1100_0
        // 1280	0110_1100_1
        (9, 1216),
        (9, 1280),
        // 1344	0110_1101_0
        // 1408	0110_1101_1
        (9, 1344),
        (9, 1408),
        //256	    0110_111
        (7, 256),
        (7, 256),
        (7, 256),
        (7, 256),
        //2	    0111
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        (4, 2),
        // 3    1000
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        (4, 3),
        //128	    1001_0
        (5, 128),
        (5, 128),
        (5, 128),
        (5, 128),
        (5, 128),
        (5, 128),
        (5, 128),
        (5, 128),
        (5, 128),
        (5, 128),
        (5, 128),
        (5, 128),
        (5, 128),
        (5, 128),
        (5, 128),
        (5, 128),
        //8	        1001_1
        (5, 8),
        (5, 8),
        (5, 8),
        (5, 8),
        (5, 8),
        (5, 8),
        (5, 8),
        (5, 8),
        (5, 8),
        (5, 8),
        (5, 8),
        (5, 8),
        (5, 8),
        (5, 8),
        (5, 8),
        (5, 8),
        //9	    1010_0
        (5, 9),
        (5, 9),
        (5, 9),
        (5, 9),
        (5, 9),
        (5, 9),
        (5, 9),
        (5, 9),
        (5, 9),
        (5, 9),
        (5, 9),
        (5, 9),
        (5, 9),
        (5, 9),
        (5, 9),
        (5, 9),
        //16  	1010_10
        (6, 16),
        (6, 16),
        (6, 16),
        (6, 16),
        (6, 16),
        (6, 16),
        (6, 16),
        (6, 16),
        //17	1010_11
        (6, 17),
        (6, 17),
        (6, 17),
        (6, 17),
        (6, 17),
        (6, 17),
        (6, 17),
        (6, 17),
        // 4	    1011
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        (4, 4),
        // 5	    1100
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        (4, 5),
        // 1101
        //    14	    1101_00
        (6, 14),
        (6, 14),
        (6, 14),
        (6, 14),
        (6, 14),
        (6, 14),
        (6, 14),
        (6, 14),
        //    15	    1101_01
        (6, 15),
        (6, 15),
        (6, 15),
        (6, 15),
        (6, 15),
        (6, 15),
        (6, 15),
        (6, 15),
        //    64	    1101_1
        (5, 64),
        (5, 64),
        (5, 64),
        (5, 64),
        (5, 64),
        (5, 64),
        (5, 64),
        (5, 64),
        (5, 64),
        (5, 64),
        (5, 64),
        (5, 64),
        (5, 64),
        (5, 64),
        (5, 64),
        (5, 64),
        //6	    1110
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        (4, 6),
        //7	    1111
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
        (4, 7),
    ];

    HuffmanTree {
        working_bits,
        max_bits,
        append_bits,
        matrix: matrix.to_vec(),
        append: append.to_vec(),
    }
}
