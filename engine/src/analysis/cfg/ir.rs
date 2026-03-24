#[derive(Clone)]
pub struct BasicBlock {
    pub id: usize,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Clone)]
pub enum EdgeKind {
    Fallthrough,
    BranchTrue,
    BranchFalse,
}

#[derive(Clone)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub kind: EdgeKind,
}

pub struct FunctionCfg {
    pub blocks: Vec<BasicBlock>,
    pub edges: Vec<Edge>,
}
