const token = () => g.choice(
    g.repOne(wordCharacter),
    string,
    number,
);

const wordCharacter = () => g.oneOf(["a", "z"], ["A", "Z"]);

const keyword = value => g.seq(value, g.notAhead(wordCharacter));

const h = g
    .whitespace(g.choice(" ", "\n", "\r", "\t"))
    .tokens(token);

const recover = () => h.choice("}", "]", ",", value);

const value = () => h.choice(
    object,
    array,
    string,
    number,
    h.label("null", keyword("null")),
    h.label("boolean", keyword("true")),
    h.label("boolean", keyword("false")),
);

const object = () => h.label("object", h.then()(
    "{",
    h.until("}")(entry, ","),
    "}"
));

const entry = () => h.label("entry", h.then(recover)(
    h.label("key", string),
    ":",
    h.label("value", value),
));

const array = () => h.label("array", h.then()(
    "[",
    h.until("]")(value, ","),
    "]",
));

const string = () => g.label("string", g.then()(
    "\"",
    g.until("\"")(stringCharacter),
    "\"",
));

const stringCharacter = () => g.choice(
    g.noneOf("\"", "\\"),
    escapeSequence,
);

const escapeSequence = () => g.seq(
    "\\",
    g.choice(
        g.choice("\"", "\\", "/", "b", "f", "n", "r", "t"),
        g.seq(
            "u",
            recoveringHexDigit,
            recoveringHexDigit,
            recoveringHexDigit,
            recoveringHexDigit
        ),
    ),
);

const recoveringHexDigit = () => g.choice(
    hexDigit,
    g.error(g.choice(g.noneOf("\\", "\""), g.empty), hexDigit),
);

const hexDigit = () => g.oneOf(["0", "9"], ["a", "f"], ["A", "F"]);

const number = () => g.label("number", g.seq(
    g.opt("-"),
    g.choice(
        "0",
        g.seq(startDigit, g.rep(digit))
    ),
    g.opt(fractional),
    g.opt(exponent),
));

const fractional = () => g.then(recover)(".", g.repOne(digit));

const exponent = () => g.then(recover)(
    g.choice("e", "E"),
    g.opt("+", "-"),
    g.repOne(digit),
);

const startDigit = () => g.oneOf(["1", "9"]);

const digit = () => g.oneOf(["0", "9"]);

module.exports = h.then()(h.empty, value, h.eof);
