const value = () => choice(
    object,
    array,
    string,
    number,
    "null",
    "true",
    "false",
);

const object = () => seq("{", rep(entry, ","), "}");

const entry = () => seq(string, ":", value);

const array = () => seq("[", rep(value, ","), "]");

const string = () => seq("\"", rep(stringCharacter), "\"");

const stringCharacter = () => choice(
    noneOf([0, 31], [127, 255], "\"", "\\"),
    escapeSequence,
);

const escapeSequence = () => seq(
    "\\",
    choice(
        choice("\"", "\\", "/", "b", "f", "n", "r", "t"),
        seq("u", hexDigit, hexDigit, hexDigit, hexDigit),
    )
);

const hexDigit = () => oneOf(["0", "9"], ["a", "f"], ["A", "F"]);

const number = () => seq(
    opt("-"),
    choice("0", seq(startDigit, rep(digit))),
    opt(fractional),
    opt(exponent),
);

const fractional = () => seq(".", repOne(digit));

const exponent = () => seq(
    choice("e", "E"),
    opt(choice("+", "-")),
    repOne(digit),
);

const startDigit = () => oneOf(["1", "9"]);

const digit = () => oneOf(["0", "9"]);

module.exports = value;