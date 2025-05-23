#![allow(unused)]

use phf::phf_map;

pub static FONT_3X5: phf::Map<char, &[&[bool]]> = phf_map! {
    '0' => &[
        &[true, true, true],
        &[true, false, true],
        &[true, false, true],
        &[true, false, true],
        &[true, true, true],
    ],
    '1' => &[
        &[true, true, false],
        &[false, true, false],
        &[false, true, false],
        &[false, true, false],
        &[true, true, true],
    ],
    '2' => &[
        &[true, true, true],
        &[false, false, true],
        &[true, true, true],
        &[true, false, false],
        &[true, true, true],
    ],
    '3' => &[
        &[true, true, true],
        &[false, false, true],
        &[false, true, true],
        &[false, false, true],
        &[true, true, true],
    ],
    '4' => &[
        &[true, false, true],
        &[true, false, true],
        &[true, true, true],
        &[false, false, true],
        &[false, false, true],
    ],
    '5' => &[
        &[true, true, true],
        &[true, false, false],
        &[true, true, true],
        &[false, false, true],
        &[true, true, true],
    ],
    '6' => &[
        &[true, true, true],
        &[true, false, false],
        &[true, true, true],
        &[true, false, true],
        &[true, true, true],
    ],
    '7' => &[
        &[true, true, true],
        &[false, false, true],
        &[false, false, true],
        &[false, false, true],
        &[false, false, true],
    ],
    '8' => &[
        &[true, true, true],
        &[true, false, true],
        &[true, true, true],
        &[true, false, true],
        &[true, true, true],
    ],
    '9' => &[
        &[true, true, true],
        &[true, false, true],
        &[true, true, true],
        &[false, false, true],
        &[true, true, true],
    ],
    ':' => &[
        &[false],
        &[true],
        &[false],
        &[true],
        &[false],
    ],
    ' ' => &[
        &[false, false, false],
        &[false, false, false],
        &[false, false, false],
        &[false, false, false],
        &[false, false, false],
    ],
    'I' => &[
        &[true],
        &[true],
        &[true],
        &[true],
        &[true],
    ],
};

pub static FONT_3X4: phf::Map<char, &[&[bool]]> = phf_map! {
    '0' => &[
        &[true, true, true],
        &[true, false, true],
        &[true, false, true],
        &[true, true, true],
    ],
    '1' => &[
        &[true, true, false],
        &[false, true, false],
        &[false, true, false],
        &[true, true, true],
    ],
    '2' => &[
        &[true, true, true],
        &[false, false, true],
        &[true, true, false],
        &[true, true, true],
    ],
    '3' => &[
        &[true, true, true],
        &[false, true, true],
        &[false, false, true],
        &[true, true, true],
    ],
    '4' => &[
        &[true, false, true],
        &[true, false, true],
        &[true, true, true],
        &[false, false, true],
    ],
    '5' => &[
        &[true, true, true],
        &[true, true, false],
        &[false, false, true],
        &[true, true, true],
    ],
    '6' => &[
        &[true, true, true],
        &[true, false, false],
        &[true, true, true],
        &[true, true, true],
    ],
    '7' => &[
        &[true, true, true],
        &[false, false, true],
        &[false, false, true],
        &[false, false, true],
    ],
    '8' => &[
        &[true, true, true],
        &[true, true, true],
        &[true, false, true],
        &[true, true, true],
    ],
    '9' => &[
        &[true, true, true],
        &[true, true, true],
        &[false, false, true],
        &[true, true, true],
    ],
    ':' => &[
        &[false],
        &[true],
        &[false],
        &[true],
    ],
    ' ' => &[
        &[false, false, false],
        &[false, false, false],
        &[false, false, false],
        &[false, false, false],
    ],
    'I' => &[
        &[true],
        &[true],
        &[true],
        &[true],
    ],
};

pub static FONT_3X3: phf::Map<char, &[&[bool]]> = phf_map! {
    '0' => &[
        &[true, true, true],
        &[true, false, true],
        &[true, true, true],
    ],
    '1' => &[
        &[true, true, false],
        &[false, true, false],
        &[true, true, true],
    ],
    '2' => &[
        &[true, true, false],
        &[false, true, false],
        &[false, true, true],
    ],
    '3' => &[
        &[true, true, true],
        &[false, true, true],
        &[true, true, true],
    ],
    '4' => &[
        &[true, false, true],
        &[true, true, true],
        &[false, false, true],
    ],
    '5' => &[
        &[false, true, true],
        &[false, true, false],
        &[true, true, false],
    ],
    '6' => &[
        &[true, false, false],
        &[true, true, true],
        &[true, true, true],
    ],
    '7' => &[
        &[true, true, true],
        &[false, false, true],
        &[false, false, true],
    ],
    '8' => &[
        &[false, true, true],
        &[true, true, true],
        &[true, true, true],
    ],
    '9' => &[
        &[true, true, true],
        &[true, true, true],
        &[false, false, true],
    ],
    ':' => &[
        &[true],
        &[false],
        &[true],
    ],
    ' ' => &[
        &[false],
        &[false],
        &[false],
    ],
    'I' => &[
        &[true],
        &[true],
        &[true],
    ],
};

pub static FONT_2X3: phf::Map<char, &[&[bool]]> = phf_map! {
    '0' => &[
        &[false, false],
        &[true, true],
        &[true, true],
    ],
    '1' => &[
        &[false, true],
        &[false, true],
        &[false, true],
    ],
    '2' => &[
        &[true, true],
        &[false, false],
        &[false, true],
    ],
    '3' => &[
        &[true, false],
        &[false, true],
        &[true, false],
    ],
    '4' => &[
        &[true, false],
        &[true, true],
        &[false, true],
    ],
    '5' => &[
        &[true, true],
        &[false, false],
        &[true, false],
    ],
    '6' => &[
        &[true, false],
        &[true, true],
        &[true, true],
    ],
    '7' => &[
        &[true, true],
        &[false, true],
        &[false, true],
    ],
    '8' => &[
        &[true, true],
        &[true, true],
        &[true, true],
    ],
    '9' => &[
        &[true, true],
        &[true, true],
        &[false, true],
    ],
    ':' => &[
        &[true],
        &[false],
        &[true],
    ],
    ' ' => &[
        &[false],
        &[false],
        &[false],
    ],
    'I' => &[
        &[true],
        &[true],
        &[true],
    ],
};
