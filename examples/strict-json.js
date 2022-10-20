const h = g.whitespace(g.choice(" ", "\n", "\r", "\t"));

const value = () => h.choice(
    object,
    array,
    string,
    number,
    h.label("null", "null"),
    h.label("boolean", "true"),
    h.label("boolean", "false"),
);

const object = () => h.label("object", h.seq("{", h.rep(entry, ","), "}"));

const entry = () => h.label("entry", h.seq(
    h.label("key", string),
    ":",
    h.label("value", value)
));

const array = () => h.label("array", h.seq("[", h.rep(value, ","), "]"));

const string = () => h.label("string", g.seq("\"", g.rep(stringCharacter), "\""));

const stringCharacter = () => g.choice(
    g.noneOf("\"", "\\"),
    escapeSequence,
);

const escapeSequence = () => g.seq(
    "\\",
    g.choice(
        g.choice("\"", "\\", "/", "b", "f", "n", "r", "t"),
        g.seq("u", hexDigit, hexDigit, hexDigit, hexDigit),
    )
);

const hexDigit = () => g.oneOf(["0", "9"], ["a", "f"], ["A", "F"]);

const number = () => h.label("number", g.seq(
    g.opt("-"),
    g.choice("0", g.seq(startDigit, g.rep(digit))),
    g.opt(fractional),
    g.opt(exponent),
));

const fractional = () => g.seq(".", g.repOne(digit));

const exponent = () => g.seq(
    g.choice("e", "E"),
    g.opt(g.choice("+", "-")),
    g.repOne(digit),
);

const startDigit = () => g.oneOf(["1", "9"]);

const digit = () => g.oneOf(["0", "9"]);

module.exports = h.seq(h.empty, value, h.eof);