use crate::tiff::decoder::ccitt::Value;
use crate::tiff::decoder::ccitt::HuffmanTree;

pub fn black_tree() -> HuffmanTree {

    // 0000

    //     EOL	    0000_0000_000
    let tree_0000_0000 = Value::Tree8(Box::new([
        Value::EOL,
        Value::None,
        Value::None,
        Value::None,
        Value::None,
        Value::None,
        Value::None,
        Value::None,
    ]));

 
    // 1792	0000_0001_000
    let tree_0000_0001_000 = Value::Value(1792);
 
    // 1984	0000_0001_0010
    // 2048	0000_0001_0011
    let tree_0000_0001_001 = Value::Tree2(Box::new([
        Value::Value(1984),
        Value::Value(2048),
    ]));

    let tree_0000_0001_00 = Value::Tree2(Box::new([
        tree_0000_0001_000,
        tree_0000_0001_001,
    ]));

    // 2112	0000_0001_0100
    // 2176	0000_0001_0101
    // 2240	0000_0001_0110
    // 2304	0000_0001_0111
    let tree_0000_0001_01 = Value::Tree4(Box::new([
        Value::Value(2112),
        Value::Value(2176),
        Value::Value(2240),
        Value::Value(2304),
    ]));

    // 1856	0000_0001_100
    // 1920	0000_0001_101
    let tree_0000_0001_10 = Value::Tree2(Box::new([
        Value::Value(1856),
        Value::Value(1920),
    ]));

    // 2368	0000_0001_1100
    // 2432	0000_0001_1101
    // 2496	0000_0001_1110
    // 2560	0000_0001_1111
    let tree_0000_0001_11 = Value::Tree4(Box::new([
        Value::Value(2368),
        Value::Value(2432),
        Value::Value(2496),
        Value::Value(2560),
    ]));
    let tree_0000_0001 = Value::Tree4(Box::new([
        tree_0000_0001_00,
        tree_0000_0001_01,
        tree_0000_0001_10,
        tree_0000_0001_11,
    ]));

    // 0000_0010_0
    // 18	    0000_0010_00
    let tree_0000_0010_00 = Value::Value(18);

    // 52	    0000_0010_0100
    let tree_0000_0010_0100 = Value::Value(52);

    // 640	    0000_0010_0101_0
    // 704	    0000_0010_0101_1
    let tree_0000_0010_0101 = Value::Tree2(Box::new([
        Value::Value(640),
        Value::Value(704),
    ]));

    // 768	    0000_0010_0110_0
    // 832	    0000_0010_0110_1
    let tree_0000_0010_0110 = Value::Tree2(Box::new([
        Value::Value(640),
        Value::Value(704),
    ]));

    // 55	    0000_0010_0111
    let tree_0000_0010_0111 = Value::Value(55);

    let tree_0000_0010_01 = Value::Tree4(Box::new([
        tree_0000_0010_0100,
        tree_0000_0010_0101,
        tree_0000_0010_0110,
        tree_0000_0010_0111,
    ]));

    let tree_0000_0010_0 = Value::Tree2(Box::new([
        tree_0000_0010_00,
        tree_0000_0010_01,
    ]));

    // 0000_0010_1

    // 56	    0000_0010_1000
    let tree_0000_0010_1000 = Value::Value(56);

    // 1280	    0000_0010_1001_0
    // 1344	    0000_0010_1001_1

    let tree_0000_0010_1001 = Value::Tree2(Box::new([
        Value::Value(1280),
        Value::Value(1344),
    ]));


    // 1408	    0000_0010_1010_0
    // 1472	    0000_0010_1010_1
    let tree_0000_0010_1010 = Value::Tree2(Box::new([
        Value::Value(1408),
        Value::Value(1472),
    ]));

    // 59	    0000_0010_1011
    let tree_0000_0010_1011 = Value::Value(59);
    let tree_0000_0010_10 = Value::Tree4(Box::new([
        tree_0000_0010_1000,
        tree_0000_0010_1001,
        tree_0000_0010_1010,
        tree_0000_0010_1011,
    ]));

    // 60	    0000_0010_1100
    let tree_0000_0010_1100 = Value::Value(60);


    // 1536	    0000_0010_1101_0
    // 1600	    0000_0010_1101_1
    let tree_0000_0010_1101 = Value::Tree2(Box::new([
        Value::Value(1536),
        Value::Value(1600),
    ]));
    
    let tree_0000_0010_110 = Value::Tree2(Box::new([
        tree_0000_0010_1100,
        tree_0000_0010_1101,
    ]));

    // 24	    0000_0010_111
    let tree_0000_0010_111 = Value::Value(24);

    let tree_0000_0010_11 = Value::Tree2(Box::new([
        tree_0000_0010_110,
        tree_0000_0010_111,
    ]));

    let tree_0000_0010_1 = Value::Tree2(Box::new([
        tree_0000_0010_10,
        tree_0000_0010_11,
    ]));

    let tree_0000_0010 = Value::Tree2(Box::new([
        tree_0000_0010_0,
        tree_0000_0010_1,
    ]));



    // 25	    0000_0011_000
    let tree_0000_0011_000 = Value::Value(25);

    // 1664	0000_0011_0010_0
    // 1728	0000_0011_0010_1
    let tree_0000_0011_0010 = Value::Tree2(Box::new([
        Value::Value(1664),
        Value::Value(1728),
    ]));

    // 320	    0000_0011_0011
    let tree_0000_0011_0011 = Value::Value(320);

    let tree_0000_0011_001= Value::Tree2(Box::new([
        tree_0000_0011_0010,
        tree_0000_0011_0011,
    ]));

    let tree_0000_0011_00= Value::Tree2(Box::new([
        tree_0000_0011_000,
        tree_0000_0011_001,
    ]));

    // 384	    0000_0011_0100
    let tree_0000_0011_0100 = Value::Value(384);

    // 448	    0000_0011_0101
    let tree_0000_0011_0101 = Value::Value(448);
    // 512	    0000_0011_0110_0
    // 576	    0000_0011_0110_1
    let tree_0000_0011_0110= Value::Tree2(Box::new([
        Value::Value(521),
        Value::Value(576),
    ]));
    // 53	    0000_0011_0111

    let tree_0000_0011_0111 = Value::Value(53);

    let tree_0000_0011_01 = Value::Tree4(Box::new([
        tree_0000_0011_0100,
        tree_0000_0011_0101,
        tree_0000_0011_0110,
        tree_0000_0011_0111,
    ]));


    // 54	    0000_0011_1000
    let tree_0000_0011_1000 = Value::Value(54);

    // 896 	0000_0011_1001_0
    // 960	0000_0011_1001_1
    let tree_0000_0011_1001= Value::Tree2(Box::new([
        Value::Value(896),
        Value::Value(960),
    ]));

    let tree_0000_0011_100 = Value::Tree2(Box::new([
        tree_0000_0011_1000,
        tree_0000_0011_1001,
    ]));

    // 1024	0000_0011_1010_0
    // 1088	0000_0011_1010_1
    // 1152	0000_0011_1011_0
    // 1216	0000_0011_1011_1
    let tree_0000_0011_101= Value::Tree4(Box::new([
        Value::Value(1024),
        Value::Value(1088),
        Value::Value(1152),
        Value::Value(1216),
    ]));

    let tree_0000_0011_10 = Value::Tree2(Box::new([
        tree_0000_0011_100,
        tree_0000_0011_101,
    ]));

    //    64	    0000_0011_11
    let tree_0000_0011_11 = Value::Value(64);
    let tree_0000_0011 = Value::Tree4(Box::new([
        tree_0000_0011_00,
        tree_0000_0011_01,
        tree_0000_0011_10,
        tree_0000_0011_11,
    ]));

    // 13  	0000_0100
    let tree_0000_0100 = Value::Value(13);

    
    // 23	    0000_0101_000
    let tree_0000_0101_000 = Value::Value(23);

    // 50	    0000_0101_0010
    // 51	    0000_0101_0011
    let tree_0000_0101_001 = Value::Tree2(Box::new([
        Value::Value(50),
        Value::Value(51),
    ]));

    let tree_0000_0101_00 = Value::Tree2(Box::new([
        tree_0000_0101_000,
        tree_0000_0101_001,
    ]));


    // 44	    0000_0101_0100
    // 45	    0000_0101_0101
    // 46	    0000_0101_0110
    // 47	    0000_0101_0111
    let tree_0000_0101_01 = Value::Tree4(Box::new([
        Value::Value(44),
        Value::Value(45),
        Value::Value(46),
        Value::Value(47),
    ]));

    // 57	    0000_0101_1000
    // 58	    0000_0101_1001
    // 61   	0000_0101_1010
    // 256	    0000_0101_1011
    let tree_0000_0101_10 = Value::Tree4(Box::new([
        Value::Value(57),
        Value::Value(58),
        Value::Value(61),
        Value::Value(256),
    ]));

    // 16  	0000_0101_11
    let tree_0000_0101_11 = Value::Value(16);

    let tree_0000_0101 = Value::Tree4(Box::new([
        tree_0000_0101_00,
        tree_0000_0101_01,
        tree_0000_0101_10,
        tree_0000_0101_11,
    ]));

    //17	    0000_0110_00
    let tree_0000_0110_00 = Value::Value(17);

    // 48  	0000_0110_0100
    // 49  	0000_0110_0101
    // 62  	0000_0110_0110
    // 63	0000_0110_0111
    let tree_0000_0110_01 = Value::Tree4(Box::new([
        Value::Value(48),
        Value::Value(49),
        Value::Value(62),
        Value::Value(63),
    ]));

    // 30  0000_0110_1000
    // 31	    0000_0110_1001
    // 32	    0000_0110_1010
    // 33	    0000_0110_1011
    let tree_0000_0110_10 = Value::Tree4(Box::new([
        Value::Value(30),
        Value::Value(31),
        Value::Value(32),
        Value::Value(33),
    ]));

    // 40	    0000_0110_1100
    // 41	    0000_0110_1101
    let tree_0000_0110_110 = Value::Tree2(Box::new([
        Value::Value(40),
        Value::Value(41),
    ]));

    // 22	    0000_0110_111
    let tree_0000_0110_111 = Value::Value(22);

    let tree_0000_0110_11 = Value::Tree2(Box::new([
        tree_0000_0110_110,
        tree_0000_0110_111,
    ]));

    let tree_0000_0110 = Value::Tree4(Box::new([
        tree_0000_0110_00,
        tree_0000_0110_01,
        tree_0000_0110_10,
        tree_0000_0110_11,
    ]));
    // 14	    0000_0111
    let tree_0000_0111 = Value::Value(14);


    let tree_0000_10 = Value::Tree2(Box::new([
        //    10	    0000_100
        Value::Value(10),
        //    11	    0000_101
        Value::Value(11),
    ]));


    // 15  	0000_1100_0
    let tree_0000_1100_0 = Value::Value(15);

    // 128	    0000_1100_1000
    // 192	    0000_1100_1001
    // 26	    0000_1100_1010
    // 27	    0000_1100_1011
    let tree_0000_1100_10 = Value::Tree4(Box::new([
        Value::Value(128),
        Value::Value(192),
        Value::Value(26),
        Value::Value(27),
    ]));

    // 28	    0000_1100_1100
    // 29	    0000_1100_1101
    let tree_0000_1100_110 = Value::Tree2(Box::new([
        Value::Value(28),
        Value::Value(29),
    ]));

    // 19	    0000_1100_111
    let tree_0000_1100_11 = Value::Tree2(Box::new([
        tree_0000_1100_110,
        Value::Value(19),
    ]));

    let tree_0000_1100_1 = Value::Tree2(Box::new([
        tree_0000_1100_10,
        tree_0000_1100_11,
    ]));

    let tree_0000_1100 = Value::Tree2(Box::new([
        tree_0000_1100_0,
        tree_0000_1100_1,
    ]));

    
    // 20	    0000_1101_000    
    let tree_0000_1101_000 = Value::Value(20);

    // 34	    0000_1101_0010
    // 35	    0000_1101_0011
    let tree_0000_1101_001 = Value::Tree2(Box::new([
        Value::Value(34),
        Value::Value(35),
    ]));

    let tree_0000_1101_00 = Value::Tree2(Box::new([
        tree_0000_1101_000,
        tree_0000_1101_001,
    ]));


    // 36	    0000_1101_0100
    // 37	    0000_1101_0101
    // 38  	    0000_1101_0110
    // 39  	    0000_1101_0111
    let tree_0000_1101_01 = Value::Tree4(Box::new([
        Value::Value(36),
        Value::Value(37),
        Value::Value(38),
        Value::Value(39),
    ]));

    // 21	    0000_1101_100

    // 42	    0000_1101_1010
    // 43  	    0000_1101_1011
    let tree_0000_1101_101 = Value::Tree2(Box::new([
        Value::Value(42),
        Value::Value(43),
    ]));
    let tree_0000_1101_10 = Value::Tree2(Box::new([
        Value::Value(21),
        tree_0000_1101_101,
    ]));
    

    // 0	    0000_1101_11
    let tree_0000_1101_11 = Value::Value(0);

    let tree_0000_1101 = Value::Tree4(Box::new([
        tree_0000_1101_00,
        tree_0000_1101_01,
        tree_0000_1101_10,
        tree_0000_1101_11,
    ]));

    let tree_0000_110 = Value::Tree2(Box::new([
        tree_0000_1100,
        tree_0000_1101,
    ]));

    // 12  	0000_111
    let tree_0000_11 = Value::Tree2(Box::new([
        tree_0000_110,
        Value::Value(21),
    ]));

    let tree_0000_00 = Value::Tree4(Box::new([
        tree_0000_0000,
        tree_0000_0001,
        tree_0000_0010,
        tree_0000_0011,
    ]));
    let tree_0000_01 = Value::Tree4(Box::new([
        tree_0000_0100,
        tree_0000_0101,
        tree_0000_0110,
        tree_0000_0111,
    ]));

    let tree_0000 = Value::Tree4(Box::new([
        tree_0000_00,
        tree_0000_01,
        tree_0000_10,
        tree_0000_11,
    ]));


    // 0001
    // 9	000100
    // 8	000101
    let tree_0001_0 = Value::Tree2(Box::new([
        Value::Value(9),
        Value::Value(8),

    ]));

    // 7	00011

    let tree_0001 = Value::Tree2(Box::new([
        tree_0001_0,
        Value::Value(7),

    ]));

    let tree_000 = Value::Tree2(Box::new([
        tree_0000,
        tree_0001,
    ]));


    // 6	0010
    // 5	0011

    let tree_001 = Value::Tree2(Box::new([
        Value::Value(6),
        Value::Value(5),
    ]));

    let tree_00 = Value::Tree2(Box::new([
        tree_000,
        tree_001,
    ]));

    // 1	010
    let tree_010 = Value::Value(1);

    // 4	011
    let tree_011 = Value::Value(4);
    let tree_01 = Value::Tree2(Box::new([
        tree_010,
        tree_011,

    ]));
 
    // 3	10
    let tree_10 = Value::Value(10);

    // 2	11
    let tree_11 = Value::Value(11);

    HuffmanTree{
        tree: Value::Tree4(Box::new([
            tree_00,
            tree_01,
            tree_10,
            tree_11,
        ]))
    }
}

