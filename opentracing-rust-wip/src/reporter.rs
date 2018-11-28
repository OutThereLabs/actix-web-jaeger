use Span;

pub trait Reporter<'a> {
    type Span: Span<'a>;

    fn report(&self, span: &Self::Span);
}
