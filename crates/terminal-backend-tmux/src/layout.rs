use crate::prelude::*;

pub(crate) fn parse_tmux_layout(
    input: &str,
    pane_ids: &HashMap<u32, PaneId>,
) -> Option<PaneTreeNode> {
    let mut parser = LayoutParser::new(input)?;
    parser.parse_node(pane_ids)
}

pub(crate) fn fallback_tree(mut pane_ids: impl Iterator<Item = PaneId>) -> PaneTreeNode {
    let first = pane_ids
        .next()
        .map(|pane_id| PaneTreeNode::Leaf { pane_id })
        .unwrap_or_else(|| PaneTreeNode::Leaf { pane_id: PaneId::new() });

    pane_ids.fold(first, |node, pane_id| {
        PaneTreeNode::Split(PaneSplit {
            direction: SplitDirection::Vertical,
            first: Box::new(node),
            second: Box::new(PaneTreeNode::Leaf { pane_id }),
        })
    })
}

struct LayoutParser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> LayoutParser<'a> {
    fn new(input: &'a str) -> Option<Self> {
        let checksum_end = input.find(',')?;
        Some(Self { input: input.as_bytes(), pos: checksum_end + 1 })
    }

    fn parse_node(&mut self, pane_ids: &HashMap<u32, PaneId>) -> Option<PaneTreeNode> {
        self.parse_number()?;
        self.expect(b'x')?;
        self.parse_number()?;
        self.expect(b',')?;
        self.parse_number()?;
        self.expect(b',')?;
        self.parse_number()?;

        match self.peek()? {
            b',' => {
                self.pos += 1;
                let pane_index = self.parse_number()? as u32;
                pane_ids.get(&pane_index).copied().map(|pane_id| PaneTreeNode::Leaf { pane_id })
            }
            b'{' | b'[' => {
                let open = self.next()?;
                let close = if open == b'{' { b'}' } else { b']' };
                let direction = if open == b'{' {
                    SplitDirection::Vertical
                } else {
                    SplitDirection::Horizontal
                };
                let mut node = self.parse_node(pane_ids)?;
                while let Some(byte) = self.peek() {
                    if byte == close {
                        self.pos += 1;
                        break;
                    }
                    self.expect(b',')?;
                    let next = self.parse_node(pane_ids)?;
                    node = PaneTreeNode::Split(PaneSplit {
                        direction,
                        first: Box::new(node),
                        second: Box::new(next),
                    });
                }
                Some(node)
            }
            _ => None,
        }
    }

    fn parse_number(&mut self) -> Option<usize> {
        let start = self.pos;
        while let Some(byte) = self.peek() {
            if !byte.is_ascii_digit() {
                break;
            }
            self.pos += 1;
        }
        (self.pos > start)
            .then(|| std::str::from_utf8(&self.input[start..self.pos]).ok()?.parse().ok())
            .flatten()
    }

    fn expect(&mut self, expected: u8) -> Option<()> {
        (self.next()? == expected).then_some(())
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.pos += 1;
        Some(byte)
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }
}
