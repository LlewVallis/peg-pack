//! Rust code generation API

pub struct Codegen {
    buffer: String,
    indent: usize,
    new_line: bool,
}

impl Codegen {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            indent: 0,
            new_line: true,
        }
    }

    pub fn finish(mut self) -> String {
        let trimmed = self.buffer.trim_end();
        let new_len = trimmed.len();
        self.buffer.truncate(new_len);

        self.buffer.push_str("\n");
        self.buffer
    }

    pub fn line(&mut self, content: &str) {
        self.write(content);
        self.newline();
    }

    pub fn function(&mut self, signature: &str) -> Statements {
        self.line("#[allow(unused)]");
        self.write(signature);
        self.space();
        self.open_brace();

        Statements::new(self, |codegen| {
            codegen.close_brace();
            codegen.newline();
        })
    }

    fn open_brace(&mut self) {
        self.indent();
        self.write("{");
        self.newline();
    }

    fn close_brace(&mut self) {
        self.dedent();
        self.write("}");
        self.newline();
    }

    pub fn indent(&mut self) {
        self.indent += 1;
    }

    pub fn dedent(&mut self) {
        self.indent -= 1;
    }

    pub fn newline(&mut self) {
        self.trim_spaces();
        self.write("\n");
        self.new_line = true;
    }

    fn space(&mut self) {
        self.write(" ");
    }

    fn trim_spaces(&mut self) {
        let trimmed = self.buffer.trim_end_matches(" ");
        let new_length = trimmed.len();
        self.buffer.truncate(new_length);
    }

    fn write(&mut self, content: &str) {
        if self.new_line {
            for _ in 0..self.indent {
                self.buffer.push_str("    ");
            }

            self.new_line = false;
        }

        self.buffer.push_str(content);
    }
}

pub struct Statements<'a> {
    codegen: &'a mut Codegen,
    finish: Option<fn(&mut Codegen)>,
}

impl<'a> Statements<'a> {
    fn new(codegen: &'a mut Codegen, finish: fn(&mut Codegen)) -> Self {
        Self {
            codegen,
            finish: Some(finish),
        }
    }

    pub fn line(&mut self, content: &str) {
        self.codegen.line(content)
    }

    pub fn match_statement(&mut self, control: &str) -> Match {
        Match::new(self.codegen, control)
    }

    pub fn if_statement(&mut self, control: &str) -> Statements {
        self.codegen.write("if ");
        self.codegen.write(control);
        self.codegen.space();
        self.codegen.open_brace();

        Statements::new(self.codegen, Codegen::close_brace)
    }
}

impl<'a> Drop for Statements<'a> {
    fn drop(&mut self) {
        let finish = self.finish.take().unwrap();
        finish(self.codegen);
    }
}

pub struct Match<'a> {
    codegen: &'a mut Codegen,
}

impl<'a> Match<'a> {
    fn new(codegen: &'a mut Codegen, control: &str) -> Self {
        codegen.write("match ");
        codegen.write(control);
        codegen.space();
        codegen.open_brace();

        Self { codegen }
    }

    pub fn case_line(&mut self, pattern: &str, line: &str) {
        self.codegen.write(pattern);
        self.codegen.write(" => ");
        self.codegen.write(line);
        self.codegen.line(",");
    }
}

impl<'a> Drop for Match<'a> {
    fn drop(&mut self) {
        self.codegen.close_brace();
    }
}
