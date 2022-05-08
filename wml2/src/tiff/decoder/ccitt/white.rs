
use crate::tiff::decoder::ccitt::Value;
use crate::tiff::decoder::ccitt::HuffmanTree;

pub fn white_tree() -> HuffmanTree {

    //EOL  	0000_0000_0001
    let tree_0000_0000 = Value::Tree(Box::new([
        Value::None,
        Value::EOL,
        Value::None,
        Value::None,
        Value::None,
        Value::None,
        Value::None,
        Value::None,
        Value::None,
        Value::None,
        Value::None,
        Value::None,
        Value::None,
        Value::None,
        Value::None,
        Value::None,
    ]));

    // 1792    0000_0001_000

    let tree_0000_0001_000 = Value::Value(1792);

    // 1984    0000_0001_0010
    // 2048    0000_0001_0011

    let tree_0000_0001_001 = Value::Tree2(Box::new([
        Value::Value(1984),
        Value::Value(2048),
    ]));

    let tree_0000_0001_00 = Value::Tree2(Box::new([
        tree_0000_0001_000,
        tree_0000_0001_001,
    ]));

    // 2112    0000_0001_0100
    // 2176    0000_0001_0101
    // 2240    0000_0001_0110
    // 2304    0000_0001_0111
    let tree_0000_0001_01 = Value::Tree4(Box::new([
        Value::Value(2112),
        Value::Value(2176),
        Value::Value(2240),
        Value::Value(2304),
    ]));

    // 1856    0000_0001_100
    // 1920    0000_0001_101
    let tree_0000_0001_10 = Value::Tree2(Box::new([
        Value::Value(1856),
        Value::Value(1920),
    ]));

    // 2368    0000_0001_1100
    // 2432    0000_0001_1101
    // 2496    0000_0001_1110
    // 2560    0000_0001_1111
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

    let tree_0000_000 = Value::Tree2(Box::new([
        tree_0000_0000,
        tree_0000_0001,
    ]));



    // 29	    0000_0010	
    // 30	    0000_0011	

    let tree_0000_001 = Value::Tree2(Box::new([
        Value::Value(29),
        Value::Value(30),
    ]));

    let tree_0000_00 = Value::Tree2(Box::new([
        tree_0000_000,
        tree_0000_001,
    ]));


    // 45	    0000_0100	
    // 46	    0000_0101	    
    let tree_0000_010 = Value::Tree2(Box::new([
        Value::Value(45),
        Value::Value(46),
    ]));

    // 22	    0000_011
    let tree_0000_01 = Value::Tree2(Box::new([
        tree_0000_010,
        Value::Value(22),
    ]));


    // 23	    0000_100	
    // 47	    0000_1010	
    // 48	    0000_1011	
    let tree_0000_101 = Value::Tree2(Box::new([
        Value::Value(47),
        Value::Value(48),
    ]));

    // 23 0000_100
    let tree_0000_10 = Value::Tree2(Box::new([
        Value::Value(23),
        tree_0000_101,
    ]));

    // 23 0000_11

    let tree_0000 = Value::Tree4(Box::new([
        tree_0000_00,
        tree_0000_01,
        tree_0000_10,
        Value::Value(13),
    ]));

    // 20	    0001_000	
    let tree_0001_000 = Value::Value(1);

    // 33	    0001_0010	
    // 34	    0001_0011	
    let tree_0001_001 = Value::Tree2(Box::new([
        Value::Value(33),        
        Value::Value(34),
    ]));

    let tree_0001_00 = Value::Tree2(Box::new([
        tree_0001_000,
        tree_0001_001,
    ]));

    // 35	    0001_0100	
    // 36	    0001_0101	
    // 37	    0001_0110	
    // 38	    0001_0111	
    let tree_0001_01 = Value::Tree4(Box::new([
        Value::Value(35),        
        Value::Value(36),
        Value::Value(37),
        Value::Value(38),
    ]));

    // 19	    0001_100	
    let tree_0001_100 = Value::Value(19);

    // 31	    0001_1010	
    // 32	    0001_1011	
    let tree_0001_101 = Value::Tree2(Box::new([
        Value::Value(31),        
        Value::Value(32),
    ]));

    let tree_0001_10 = Value::Tree2(Box::new([
        tree_0001_100,
        tree_0001_101,
    ]));


    //  1	    0001_11		
    let tree_0001_11 = Value::Value(1);

    let tree_0001 = Value::Tree4(Box::new([
        tree_0001_00,
        tree_0001_01,
        tree_0001_10,
        tree_0001_11,
    ]));

    // 12	    0010_00		
    let tree_0010_00 = Value::Value(12);
    // 53	    0010_0100	
    // 54	    0010_0101	
    let tree_0010_010 = Value::Tree2(Box::new([
        Value::Value(53),
        Value::Value(54),
    ]));

    // 26	    0010_011	

    let tree_0010_01 = Value::Tree2(Box::new([
        tree_0010_010,
        Value::Value(26),
    ]));


    // 39	    0010_1000	
    // 40	    0010_1001	
    // 41	    0010_1010	
    // 42	    0010_1011	
    let tree_0010_10 = Value::Tree4(Box::new([
        Value::Value(39),
        Value::Value(40),
        Value::Value(41),
        Value::Value(42),
    ]));

    // 43  	    0010_1100	
    // 44	    0010_1101	
    let tree_0010_110 = Value::Tree2(Box::new([
        Value::Value(43),
        Value::Value(44),
    ]));

    // 21	    0010_111	
    let tree_0010_11 = Value::Tree2(Box::new([
        tree_0010_110,
        Value::Value(21),
    ]));


    let tree_0010 = Value::Tree4(Box::new([
        tree_0010_00,
        tree_0010_01,
        tree_0010_10,
        tree_0010_11,
    ]));



    // 61  	    0011_0010	
    // 62	    0011_0011	

    let tree_0011_001 = Value::Tree2(Box::new([
        Value::Value(61),
        Value::Value(62),
    ]));

    // 28	    0011_000	
    let tree_0011_00 = Value::Tree2(Box::new([
        Value::Value(28),
        tree_0011_001,
    ]));

    // 63	    0011_0100	
    // 0	    0011_0101
    // 320	    0011_0110	
    // 384	    0011_0111
    
    let tree_0011_01 = Value::Tree4(Box::new([
        Value::Value(63),
        Value::Value(0),
        Value::Value(320),
        Value::Value(384),
    ]));

    let tree_0011_0 = Value::Tree2(Box::new([
        tree_0011_00,
        tree_0011_01,
    ]));

    // 10	    0011_1
    let tree_0011 =Value::Tree2(Box::new([
        tree_0011_0,
        Value::Value(10),
    ]));


    // 01000

    // 0100_101
    // 59	    0100_1010	
    // 60	    0100_1011
    let tree_0100_101 = Value::Tree2(Box::new([
        Value::Value(59),
        Value::Value(60),
    ]));

    // 27	    0100_100	
    let tree_0100_10 = Value::Tree2(Box::new([
        Value::Value(27),
        tree_0100_101,
    ]));

    // 0100_11
    // 1472	0100_11000	
    // 1536	0100_11001	
    // 1600	0100_11010	
    // 1728	0100_11011

    let tree_0100_110 = Value::Tree4(Box::new([
        Value::Value(1472),
        Value::Value(1536),
        Value::Value(1600),
        Value::Value(1728),
    ]));
    // 0100_111
    // 18	    0100_111

    let tree_0100_11 = Value::Tree2(Box::new([
        tree_0100_110,
        Value::Value(18),
    ]));
    let tree_0100_1 = Value::Tree2(Box::new([
        tree_0100_10,
        tree_0100_11,
    ]));

    // 11	    0100_0
    let tree_0100 = Value::Tree2(Box::new([
        Value::Value(11),
        tree_0100_1,
    ]));



    // 0101

    //  24	    0101_000	
    let tree_0101_000 = Value::Value(24);

    // 49	    0101_0010
    // 50	    0101_0011	

    let tree_0101_001 = Value::Tree2(Box::new([
            Value::Value(49),
            Value::Value(50)
    ]));

    let tree_0101_00 = Value::Tree2(Box::new([
            tree_0101_000,
            tree_0101_001,
    ]));

    // 51	    0101_0100
    // 52	    0101_0101	

    let tree_0101_010 = Value::Tree2(Box::new([
            Value::Value(704),
            Value::Value(768)
    ]));
    // 25	    0101_011
 
    let tree_0101_01 = Value::Tree2(Box::new([
        tree_0101_010,
        Value::Value(25),    
    ]));


    // 55	    0101_1000	
    // 56	    0101_1001	
    // 57	    0101_1010	
    // 58	    0101_1011	

    let tree_0101_10 = Value::Tree4(Box::new([
        Value::Value(55),
        Value::Value(56),
        Value::Value(57),
        Value::Value(58),
    ]));

    // 192 0101_10
    let tree_0101 = Value::Tree4(Box::new([
        tree_0101_00,
        tree_0101_01,
        tree_0101_10,
        Value::Value(192),
    ]));


    // 1664	0110_00
    let tree_0110_00 = Value::Value(1664);

/*
    448	    0110_0100	
    512	    0110_0101	
    704     0110_0110_0
    768	    0110_0110_1	
*/
    let tree_0110_0100 = Value::Value(448);
    let tree_0110_0101 = Value::Value(512);
    let tree_0110_0110 = Value::Tree2(Box::new(
         [
            Value::Value(704),
            Value::Value(768)
        ]));
    let tree_0110_0111 = Value::Value(640);

    let tree_0110_01 = Value::Tree4(Box::new(
         [
            tree_0110_0100,
            tree_0110_0101,
            tree_0110_0110,
            tree_0110_0111,
        ]));


    // 0110_10
    //    576	    0110_1000
    let tree_0110_1000 = Value::Value(576);

    // 832	    0110_1001_0	
    // 896	    0110_1001_1

    let tree_0110_1001 = Value::Tree2(Box::new(
        [
            Value::Value(832),
            Value::Value(896)
        ]));

    let tree_0110_100 = Value::Tree2(Box::new([
            tree_0110_1000,
            tree_0110_1001,
        ]));


    /*
    960	    0110_1010_0	
    1024	0110_1010_1	
    1088	0110_1011_0	
    1152	0110_1011_1	
  */
    let tree_0110_101 = Value::Tree4(Box::new(
         [
            Value::Value(960),
            Value::Value(1024),
            Value::Value(1088),
            Value::Value(1152),
        ]));

    let tree_0110_10 = Value::Tree2(Box::new(
         [
            tree_0110_100,
            tree_0110_101,
        ]));

    /*
    // 0110_110
    1216	0110_1100_0	
    1280	0110_1100_1	
    1344	0110_1101_0
    1408	0110_1101_1	
    */

    let tree_0110_110 = Value::Tree4(Box::new([
            Value::Value(1216),
            Value::Value(1280),
            Value::Value(1344),
            Value::Value(1408),
        ]));

    //256	    0110_111	

    let tree_0110_11 = Value::Tree2(Box::new([
        tree_0110_110,
        Value::Value(256),
    ]));


    let tree_0110 = Value::Tree4(Box::new([
        tree_0110_00,
        tree_0110_01,
        tree_0110_10,
        tree_0110_11,
    ]));

    //2	    0111
    let tree_0111 = Value::Value(2);

    // 3    1000
    let tree_1000 = Value::Value(4);

    //128	    1001_0
    //8	        1001_1
    let tree_1001 = Value::Tree2(Box::new([
        Value::Value(128),
        Value::Value(8),
    ]));
    
    //16  	1010_10		
    //17	1010_11
    let tree_1010_1 = Value::Tree2(Box::new([
        Value::Value(16),
        Value::Value(17),
    ]));

    //9	    1010_0
    let tree_1010 = Value::Tree2(Box::new([
        Value::Value(9),
        tree_1010_1
    ]));

    // 4	    1011
    let tree_1011 = Value::Value(4);


    // 5	    1100
    let tree_1100 = Value::Value(5);

    // 1101
    /*
        14	    1101_00		
        15	    1101_01		
    */

    let tree_1101_0 = Value::Tree2(Box::new([
            Value::Value(14),
            Value::Value(15),
    ]));

    /*
        64	    1101_1
    */

    let tree_1101 = Value::Tree2(Box::new([
            tree_1101_0,
            Value::Value(64),
    ]));


    //6	    1110
    let tree_1110 = Value::Value(6);

    //7	    1111
    let tree_1111 = Value::Value(7);


    HuffmanTree {
        tree:Value::Tree(Box::new([
            tree_0000,
            tree_0001,
            tree_0010,
            tree_0011,
            tree_0100,
            tree_0101,
            tree_0110,
            tree_0111,
            tree_1000,
            tree_1001,
            tree_1010,
            tree_1011,
            tree_1100,
            tree_1101,
            tree_1110,
            tree_1111,
        ]))
    }
} 
