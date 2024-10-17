// -*- mode: Bluespec; -*-

module node {
    import hashes.* from "./hashes"

type Option[a] = 
    | None
    | Some(a)

type Child = {
    version: int, //u64 //TODO: Version not hashed
    hash: Term_t,
}

type InternalNode = {
    left_child: Option[Child],
    right_child: Option[Child],
}

type LeafNode = {
        // In the implementation it is a hash of a key but in the
        // Radix tree it is just used as a key, so we use a list
        // of bit and
        // we treat it here just as bytes    
    key_hash: Bytes_t, 
    value_hash: Bytes_t,
}

type Node = 
    | Internal(InternalNode)
    | Leaf(LeafNode)

pure def is_leaf(n: Node) : bool =
    match n {
        | Leaf(n) => true
        | Internal(n) => false
}
val InternalNodeIdentifier : Bytes_t = [0]
val LeafNodeIdentifier : Bytes_t = [1]
val InternalNodeHashPrefix : Term_t = raw(InternalNodeIdentifier)
val LeafNodeHashPrefix : Term_t = raw(LeafNodeIdentifier)

val l = {
    key_hash: termHash(raw([2])),
    value_hash:  termHash(raw([17]))
}
pure def hashLeafNode(l: LeafNode): Term_t =
    termHash(   LeafNodeHashPrefix
                .termConcat(raw(l.key_hash))
                .termConcat(raw(l.value_hash)))

pure def hashOfChild(oc: Option[Child]) : Term_t =
    match oc {
        | None => termHash(raw([0]))
        | Some(c) => c.hash
    }

pure def hashInternalNode(l: InternalNode): Term_t =
    termHash(   InternalNodeHashPrefix
                .termConcat(l.left_child.hashOfChild())
                .termConcat(l.right_child.hashOfChild()))

pure def hash(n: Node) : Term_t = {
    match n {
        | Leaf(n) => hashLeafNode(n)
        | Internal(n) => hashInternalNode(n)
    }
}

// runs
run hash_InternalTest = {
    nondet left_child_key = 0.to(16).oneOf()
    nondet left_child_value = 0.to(16).oneOf()
    val left_leaf = {
        key_hash: [left_child_key],
        value_hash:  [left_child_value]
    }

    nondet right_child_key = 0.to(16).oneOf()
    nondet right_child_value = 0.to(16).oneOf()
    val right_leaf = {
        key_hash: [right_child_key],
        value_hash: [right_child_value]
    }

    val left_leaf_hash = hashLeafNode(left_leaf)
    val right_leaf_hash = hashLeafNode(right_leaf)
// visual representation of hashed leaf node    
//                              *
//                              | [0]
//                              Hash
//                      /[0,0]   |[0,1]   \ [0,2]
//  Raw(LeafNodeIdentifier)     Hash      Hash
//                       [0,1,0]/           / [0,2,0]
//            Raw([left_child_key])    Raw([left_child_value])

    val expected_left_hash =  Map([0] -> Hash,
                            [0, 0] -> Raw(LeafNodeIdentifier),
                            [0, 1] -> Hash, 
                                [0, 1, 0] -> Raw([left_child_key]), 
                            [0, 2] -> Hash,        
                                [0, 2, 0] -> Raw([left_child_value])
                )
    val expected_right_hash =  Map([0] -> Hash,
                            [0, 0] -> Raw(LeafNodeIdentifier),
                            [0, 1] -> Hash, 
                                [0, 1, 0] -> Raw([right_child_key]), 
                            [0, 2] -> Hash,        
                                [0, 2, 0] -> Raw([right_child_value])
    )
    
    val internal_node = {
        left_child: Some({version: 0, hash: left_leaf_hash}),
        right_child: Some({version: 0, hash: right_leaf_hash})
    }    
    val hashed_internal_node = hash(Internal(internal_node))
// visual representation of hashed internal node    
//                                                          *
//                                                          | [0]
//                                                        Hash
//                            /                             |                                      \
//                           /                              |                                       \
//                          /                               |                                        \
//                         /                                |                                         \
//                        /                                 |                                          \
//                       /                                  |                                           \
//                      /[0,0]                              |[0,1]                                       \ [0,2]
//  Raw(InternalNodeIdentifier)                           Hash                                           Hash
//                                       [0,1,0]/    [0,1,1]|   [0,1,2]\              [0,2,0] /    [0,2,1]|   [0,2,2]\
//                          Raw(LeafNodeIdentifier)       Hash       Hash      Raw(LeafNodeIdentifier)   Hash       Hash
//                                              [0,1,1,0]/           / [0,1,2,0]                [0,2,1,0]/           / [0,2,2,0]
//                                  Raw([left_child_key])   Raw([left_child_value])     Raw([right_child_key])   Raw([righ_child_value])
    
    val expected_internal_node_hash =  Map([0] -> Hash,
                            [0,0] -> Raw(InternalNodeIdentifier),
                            [0, 1] -> Hash,
                                [0, 1, 0] -> Raw(LeafNodeIdentifier),
                                [0, 1, 1] -> Hash, 
                                    [0, 1, 1, 0] -> Raw([left_child_key]), 
                                [0, 1, 2] -> Hash,        
                                    [0, 1, 2, 0] -> Raw([left_child_value]),
                            [0, 2] -> Hash,
                                [0, 2, 0] -> Raw(LeafNodeIdentifier),
                                [0, 2, 1] -> Hash, 
                                    [0, 2, 1, 0] -> Raw([right_child_key]), 
                                [0, 2, 2] -> Hash,        
                                    [0, 2, 2, 0] -> Raw([right_child_value])
                            )

    assert(hashed_internal_node == expected_internal_node_hash)
}
run hash_LeafTest = {
    nondet key = 0.to(16).oneOf()
    nondet value = 0.to(16).oneOf()
    val l = {
        key_hash: [key],
        value_hash:  [value]
    }
    val result = hash(Leaf(l))

    val expected =  Map([0] -> Hash,
                            [0, 0] -> Raw(LeafNodeIdentifier),
                            [0, 1] -> Hash, 
                                [0, 1, 0] -> Raw([key]), 
                            [0, 2] -> Hash,        
                                [0, 2, 0] -> Raw([value]) 
                )
    assert(result == expected)
}
run LeafHashTest = {  
    nondet key = 0.to(16).oneOf()
    nondet value = 0.to(16).oneOf()
    val l = {
        key_hash: [key],
        value_hash: [value]
    }
    val result = hashLeafNode(l)
    val expected =  Map([0] -> Hash,
                            [0, 0] -> Raw(LeafNodeIdentifier),
                            [0, 1] -> Hash, 
                                [0, 1, 0] -> Raw([key]), 
                            [0, 2] -> Hash,        
                                [0, 2, 0] -> Raw([value]) 
                )
    assert(result == expected)
}

run InternalHashTest = {
    nondet left_child_key = 0.to(16).oneOf()
    nondet left_child_value = 0.to(16).oneOf()
    val left_leaf = {
        key_hash: [left_child_key],
        value_hash:  [left_child_value]
    }

    nondet right_child_key = 0.to(16).oneOf()
    nondet right_child_value = 0.to(16).oneOf()
    val right_leaf = {
        key_hash: [right_child_key],
        value_hash: [right_child_value]
    }

    val left_leaf_hash = hashLeafNode(left_leaf)
    val right_leaf_hash = hashLeafNode(right_leaf)
    
    val expected_left_hash =  Map([0] -> Hash,
                            [0, 0] -> Raw(LeafNodeIdentifier),
                            [0, 1] -> Hash, 
                                [0, 1, 0] -> Raw([left_child_key]), 
                            [0, 2] -> Hash,        
                                [0, 2, 0] -> Raw([left_child_value])
                            )
    val expected_right_hash =  Map([0] -> Hash,
                            [0, 0] -> Raw(LeafNodeIdentifier),
                            [0, 1] -> Hash, 
                                [0, 1, 0] -> Raw([right_child_key]), 
                            [0, 2] -> Hash,        
                                [0, 2, 0] -> Raw([right_child_value])
                            )
    
    val internal_node = {
        left_child: Some({version: 0, hash: left_leaf_hash}),
        right_child: Some({version: 0, hash: right_leaf_hash})
    }    
    val hashed_internal_node = hashInternalNode(internal_node)
    val expected_internal_node_hash =  Map([0] -> Hash,
                            [0,0] -> Raw(InternalNodeIdentifier),
                            [0, 1] -> Hash,
                                [0, 1, 0] -> Raw(LeafNodeIdentifier),
                                [0, 1, 1] -> Hash, 
                                    [0, 1, 1, 0] -> Raw([left_child_key]), 
                                [0, 1, 2] -> Hash,        
                                    [0, 1, 2, 0] -> Raw([left_child_value]),
                            [0, 2] -> Hash,
                                [0, 2, 0] -> Raw(LeafNodeIdentifier),
                                [0, 2, 1] -> Hash, 
                                    [0, 2, 1, 0] -> Raw([right_child_key]), 
                                [0, 2, 2] -> Hash,        
                                    [0, 2, 2, 0] -> Raw([right_child_value])
                            )

    assert(left_leaf_hash == expected_left_hash).then(assert(right_leaf_hash == expected_right_hash)).then(assert(expected_internal_node_hash == hashed_internal_node))

}
run LeafCheckIsLeafTest = {
    nondet key = 0.to(16).oneOf()
    nondet value = 0.to(16).oneOf()
    val l = {
        key_hash: [key],
        value_hash:  [value]
    }
    assert(is_leaf(Leaf(l)) == true)
}

run InternalNodeCheckIsLeafTest = {
    nondet left_child_key = 0.to(16).oneOf()
    nondet left_child_value = 0.to(16).oneOf()
    val left_leaf = {
        key_hash: [left_child_key],
        value_hash:  [left_child_value]
    }

    nondet right_child_key = 0.to(16).oneOf()
    nondet right_child_value = 0.to(16).oneOf()
    val right_leaf = {
        key_hash: [right_child_key],
        value_hash:  [right_child_value]
    }

    val left_leaf_hash = hashLeafNode(left_leaf)
    val right_leaf_hash = hashLeafNode(right_leaf)

    val internal_node = {
        left_child: Some({version: 0, hash: left_leaf_hash}),
        right_child: Some({version: 0, hash: right_leaf_hash})
    }

    assert(is_leaf(Internal(internal_node)) == false)
}
run IncompleteInternalNodeCheckIsLeafTest = {
    nondet left_child_key = 0.to(16).oneOf()
    nondet left_child_value = 0.to(16).oneOf()
    val left_leaf = {
        key_hash: [left_child_key],
        value_hash: [left_child_value]
    }
    val left_leaf_hash = hashLeafNode(left_leaf)

    val inner_node = {
        left_child: Some({version: 0, hash: left_leaf_hash}),
        right_child: None
    }

    assert(is_leaf(Internal(inner_node)) == false)
}

run NoneHashOfChildTest = {
    val hash_of_none = hashOfChild(None)
    val expected_hash = Map( [0] -> Hash, [0, 0] -> Raw([0]))

    assert(expected_hash == hash_of_none)
}
// TODO test more

}