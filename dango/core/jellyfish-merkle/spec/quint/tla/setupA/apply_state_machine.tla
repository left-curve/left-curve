-------------------------- MODULE apply_state_machine --------------------------

EXTENDS Integers, Sequences, FiniteSets, TLC, Apalache, Variants

VARIABLE
  (*
    @type: Int;
  *)
  version

VARIABLE
  (*
    @type: Int;
  *)
  smallest_unpruned_version

(*
  @type: ((Set(a), ((a) => None({ tag: Str }) | Some(b))) => Set(b));
*)
filterMap(s_4129, f_4129(_)) ==
  LET (*
    @type: ((Set(b), a) => Set(b));
  *)
  __QUINT_LAMBDA2(acc_4127, e_4127) ==
    CASE VariantTag((f_4129(e_4127))) = "Some"
        -> LET (*
          @type: ((b) => Set(b));
        *)
        __QUINT_LAMBDA0(x_4122) == acc_4127 \union {x_4122}
        IN
        __QUINT_LAMBDA0(VariantGetUnsafe("Some", (f_4129(e_4127))))
      [] VariantTag((f_4129(e_4127))) = "None"
        -> LET (*
          @type: (({ tag: Str }) => Set(b));
        *)
        __QUINT_LAMBDA1(id__4125) == acc_4127
        IN
        __QUINT_LAMBDA1(VariantGetUnsafe("None", (f_4129(e_4127))))
  IN
  ApaFoldSet(__QUINT_LAMBDA2, {}, s_4129)

(*
  @type: (((c -> d)) => Set(<<c, d>>));
*)
mapToTuples(m_4280) == { <<k_4278, m_4280[k_4278]>>: k_4278 \in DOMAIN m_4280 }

(*
  @type: ((e) => None({ tag: Str }) | Some(e));
*)
Some(__SomeParam_1241) == Variant("Some", __SomeParam_1241)

(*
  @type: (() => None({ tag: Str }) | Some(f));
*)
None == Variant("None", [tag |-> "UNIT"])

(*
  @type: ((Seq(Int)) => Delete({ tag: Str }) | Insert(Seq(Int)));
*)
Insert(__InsertParam_3914) == Variant("Insert", __InsertParam_3914)

(*
  @type: (() => Delete({ tag: Str }) | Insert(Seq(Int)));
*)
Delete == Variant("Delete", [tag |-> "UNIT"])

VARIABLE
  (*
    @type: { nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) };
  *)
  tree

VARIABLE
  (*
    @type: Seq(Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }));
  *)
  ops_history

(*
  @type: ((Seq(Int)) => Bool);
*)
isNil(w_2897) == w_2897 = <<>>

(*
  @type: ((Seq(Int), Seq(Int)) => Bool);
*)
less_than(w1_2824, w2_2824) ==
  LET (*@type: (() => Int); *) len1 == Len(w1_2824) IN
  LET (*@type: (() => Int); *) len2 == Len(w2_2824) IN
  (len1 < len2
      /\ (\A i_2785 \in LET (*
        @type: (() => Set(Int));
      *)
      __quint_var0 == DOMAIN w1_2824
      IN
      IF __quint_var0 = {}
      THEN {}
      ELSE (__quint_var0 \union {0}) \ {(Len(w1_2824))}:
        w1_2824[(i_2785 + 1)] <= w2_2824[(i_2785 + 1)]))
    \/ (len1 = len2
      /\ (\E i_2818 \in LET (*
        @type: (() => Set(Int));
      *)
      __quint_var1 == DOMAIN w1_2824
      IN
      IF __quint_var1 = {}
      THEN {}
      ELSE (__quint_var1 \union {0}) \ {(Len(w1_2824))}:
        w1_2824[(i_2818 + 1)] < w2_2824[(i_2818 + 1)]
          /\ (\A j_2815 \in LET (*
            @type: (() => Set(Int));
          *)
          __quint_var2 == DOMAIN w1_2824
          IN
          IF __quint_var2 = {}
          THEN {}
          ELSE (__quint_var2 \union {0}) \ {(Len(w1_2824))}:
            j_2815 < i_2818 => w1_2824[(j_2815 + 1)] = w2_2824[(j_2815 + 1)])))

(*
  @type: ((Seq(Int), Seq(Int)) => Bool);
*)
greater_than(w1_2889, w2_2889) ==
  LET (*@type: (() => Int); *) len1 == Len(w1_2889) IN
  LET (*@type: (() => Int); *) len2 == Len(w2_2889) IN
  (len1 > len2
      /\ (\A i_2850 \in LET (*
        @type: (() => Set(Int));
      *)
      __quint_var3 == DOMAIN w1_2889
      IN
      IF __quint_var3 = {}
      THEN {}
      ELSE (__quint_var3 \union {0}) \ {(Len(w1_2889))}:
        w1_2889[(i_2850 + 1)] <= w2_2889[(i_2850 + 1)]))
    \/ (len1 = len2
      /\ (\E i_2883 \in LET (*
        @type: (() => Set(Int));
      *)
      __quint_var4 == DOMAIN w1_2889
      IN
      IF __quint_var4 = {}
      THEN {}
      ELSE (__quint_var4 \union {0}) \ {(Len(w1_2889))}:
        w1_2889[(i_2883 + 1)] > w2_2889[(i_2883 + 1)]
          /\ (\A j_2880 \in LET (*
            @type: (() => Set(Int));
          *)
          __quint_var5 == DOMAIN w1_2889
          IN
          IF __quint_var5 = {}
          THEN {}
          ELSE (__quint_var5 \union {0}) \ {(Len(w1_2889))}:
            j_2880 < i_2883 => w1_2889[(j_2880 + 1)] = w2_2889[(j_2880 + 1)])))

(*
  @type: (() => Hash({ tag: Str }) | Raw(Seq(Int)));
*)
Hash == Variant("Hash", [tag |-> "UNIT"])

(*
  @type: ((Seq(Int)) => Hash({ tag: Str }) | Raw(Seq(Int)));
*)
Raw(__RawParam_2911) == Variant("Raw", __RawParam_2911)

(*
  @type: (((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))) => Int);
*)
termLen(term_2958) ==
  LET (*
    @type: (() => Set(Hash({ tag: Str }) | Raw(Seq(Int))));
  *)
  top ==
    {
      term_2958[p_2933]:
        p_2933 \in { p_2927 \in DOMAIN term_2958: Len(p_2927) = 1 }
    }
  IN
  LET (*
    @type: ((Int, Hash({ tag: Str }) | Raw(Seq(Int))) => Int);
  *)
  __QUINT_LAMBDA8(s_2955, t_2955) ==
    CASE VariantTag(t_2955) = "Hash"
        -> LET (*
          @type: (({ tag: Str }) => Int);
        *)
        __QUINT_LAMBDA6(id__2950) == s_2955 + 32
        IN
        __QUINT_LAMBDA6(VariantGetUnsafe("Hash", t_2955))
      [] VariantTag(t_2955) = "Raw"
        -> LET (*
          @type: ((Seq(Int)) => Int);
        *)
        __QUINT_LAMBDA7(bytes_2953) == s_2955 + Len(bytes_2953)
        IN
        __QUINT_LAMBDA7(VariantGetUnsafe("Raw", t_2955))
  IN
  ApaFoldSet(__QUINT_LAMBDA8, 0, (top))

(*
  @type: ((Seq(Int)) => Hash({ tag: Str }) | Raw(Seq(Int)));
*)
fancy_Raw(fancy___RawParam_2911) == Variant("Raw", fancy___RawParam_2911)

(*
  @type: (() => Hash({ tag: Str }) | Raw(Seq(Int)));
*)
fancy_Hash == Variant("Hash", [tag |-> "UNIT"])

(*
  @type: (() => Seq(Int));
*)
InternalNodeIdentifier == <<0>>

(*
  @type: (() => Seq(Int));
*)
LeafNodeIdentifier == <<1>>

(*
  @type: (() => Seq(Int));
*)
fancy_InternalNodeIdentifier == <<0>>

(*
  @type: (() => Seq(Int));
*)
fancy_LeafNodeIdentifier == <<1>>

(*
  @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }));
*)
Internal(__InternalParam_3875) == Variant("Internal", __InternalParam_3875)

(*
  @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }));
*)
Leaf(__LeafParam_3881) == Variant("Leaf", __LeafParam_3881)

(*
  @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => Bool);
*)
is_leaf(n_3933) ==
  CASE VariantTag(n_3933) = "Leaf"
      -> LET (*
        @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => Bool);
      *)
      __QUINT_LAMBDA27(n_3928) == TRUE
      IN
      __QUINT_LAMBDA27(VariantGetUnsafe("Leaf", n_3933))
    [] VariantTag(n_3933) = "Internal"
      -> LET (*
        @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => Bool);
      *)
      __QUINT_LAMBDA28(n_3931) == FALSE
      IN
      __QUINT_LAMBDA28(VariantGetUnsafe("Internal", n_3933))

(*
  @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => { key_hash: Seq(Int), value_hash: Seq(Int) });
*)
getLeafOrEmpty(n_4038) ==
  CASE VariantTag(n_4038) = "Leaf"
      -> LET (*
        @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => { key_hash: Seq(Int), value_hash: Seq(Int) });
      *)
      __QUINT_LAMBDA31(l_4033) == l_4033
      IN
      __QUINT_LAMBDA31(VariantGetUnsafe("Leaf", n_4038))
    [] VariantTag(n_4038) = "Internal"
      -> LET (*
        @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => { key_hash: Seq(Int), value_hash: Seq(Int) });
      *)
      __QUINT_LAMBDA32(internal_4036) ==
        [key_hash |-> <<>>, value_hash |-> <<>>]
      IN
      __QUINT_LAMBDA32(VariantGetUnsafe("Internal", n_4038))

(*
  @type: ((None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
*)
Unchanged(__UnchangedParam_3894) == Variant("Unchanged", __UnchangedParam_3894)

(*
  @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
*)
Updated(__UpdatedParam_3900) == Variant("Updated", __UpdatedParam_3900)

(*
  @type: (() => Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
*)
Deleted == Variant("Deleted", [tag |-> "UNIT"])

(*
  @type: (() => Int);
*)
MAX_HASH_LENGTH == 3

(*
  @type: (() => Set(Seq(Int)));
*)
all_value_hashes == { <<0>>, <<1>> }

(*
  @type: (() => Seq(Int));
*)
ROOT_BITS == <<>>

(*
  @type: (({ key_hash: Seq(Int), version: Int }, Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int })) => Bool);
*)
is_node_orphaned(nodeId_3305, orphans_3305) ==
  \E el_3303 \in orphans_3305:
    el_3303["version"] = nodeId_3305["version"]
      /\ el_3303["key_hash"] = nodeId_3305["key_hash"]

(*
  @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => Bool);
*)
isInternal(n_3748) ==
  CASE VariantTag(n_3748) = "Internal"
      -> LET (*
        @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => Bool);
      *)
      __QUINT_LAMBDA33(id__3743) == TRUE
      IN
      __QUINT_LAMBDA33(VariantGetUnsafe("Internal", n_3748))
    [] VariantTag(n_3748) = "Leaf"
      -> LET (*
        @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => Bool);
      *)
      __QUINT_LAMBDA34(id__3746) == FALSE
      IN
      __QUINT_LAMBDA34(VariantGetUnsafe("Leaf", n_3748))

(*
  @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>)) => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
*)
add_nodes(nodes_3817, new_nodes_3817) ==
  LET (*
    @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), <<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>) => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
  *)
  __QUINT_LAMBDA37(nodes_3815, new_node_3815) ==
    LET (*
      @type: (() => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
    *)
    __quint_var7 == nodes_3815
    IN
    LET (*
      @type: (() => Set({ key_hash: Seq(Int), version: Int }));
    *)
    __quint_var8 == DOMAIN __quint_var7
    IN
    [
      __quint_var9 \in {new_node_3815[1]} \union __quint_var8 |->
        IF __quint_var9 = new_node_3815[1]
        THEN new_node_3815[2]
        ELSE (__quint_var7)[__quint_var9]
    ]
  IN
  ApaFoldSet(__QUINT_LAMBDA37, nodes_3817, new_nodes_3817)

(*
  @type: ((k) => None({ tag: Str }) | Some(k));
*)
fancy_Some(fancy___SomeParam_1241) == Variant("Some", fancy___SomeParam_1241)

(*
  @type: (() => None({ tag: Str }) | Some(m));
*)
fancy_None == Variant("None", [tag |-> "UNIT"])

(*
  @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Set({ key_hash: Seq(Int), value_hash: Seq(Int) }));
*)
allLeafs(t_3417) ==
  LET (*
    @type: (() => Set(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
  *)
  allNodes ==
    LET (*
      @type: ((Set(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), { key_hash: Seq(Int), version: Int }) => Set(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
    *)
    __QUINT_LAMBDA43(s_3394, x_3394) == s_3394 \union {t_3417[x_3394]}
    IN
    ApaFoldSet(__QUINT_LAMBDA43, {}, (DOMAIN t_3417))
  IN
  LET (*
    @type: ((Set({ key_hash: Seq(Int), value_hash: Seq(Int) }), Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => Set({ key_hash: Seq(Int), value_hash: Seq(Int) }));
  *)
  __QUINT_LAMBDA46(s_3414, x_3414) ==
    CASE VariantTag(x_3414) = "Internal"
        -> LET (*
          @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => Set({ key_hash: Seq(Int), value_hash: Seq(Int) }));
        *)
        __QUINT_LAMBDA44(id__3409) == s_3414
        IN
        __QUINT_LAMBDA44(VariantGetUnsafe("Internal", x_3414))
      [] VariantTag(x_3414) = "Leaf"
        -> LET (*
          @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => Set({ key_hash: Seq(Int), value_hash: Seq(Int) }));
        *)
        __QUINT_LAMBDA45(x_3412) == s_3414 \union {x_3412}
        IN
        __QUINT_LAMBDA45(VariantGetUnsafe("Leaf", x_3414))
  IN
  ApaFoldSet(__QUINT_LAMBDA46, {}, (allNodes))

(*
  @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => ({ key_hash: Seq(Int), version: Int } -> { left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }));
*)
allInternals(t_3446) ==
  LET (*
    @type: ((({ key_hash: Seq(Int), version: Int } -> { left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }), { key_hash: Seq(Int), version: Int }) => ({ key_hash: Seq(Int), version: Int } -> { left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }));
  *)
  __QUINT_LAMBDA49(internals_3444, key_3444) ==
    CASE VariantTag(t_3446[key_3444]) = "Internal"
        -> LET (*
          @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => ({ key_hash: Seq(Int), version: Int } -> { left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }));
        *)
        __QUINT_LAMBDA47(x_3439) ==
          LET (*
            @type: (() => ({ key_hash: Seq(Int), version: Int } -> { left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }));
          *)
          __quint_var10 == internals_3444
          IN
          LET (*
            @type: (() => Set({ key_hash: Seq(Int), version: Int }));
          *)
          __quint_var11 == DOMAIN __quint_var10
          IN
          [
            __quint_var12 \in {key_3444} \union __quint_var11 |->
              IF __quint_var12 = key_3444
              THEN x_3439
              ELSE (__quint_var10)[__quint_var12]
          ]
        IN
        __QUINT_LAMBDA47(VariantGetUnsafe("Internal", t_3446[key_3444]))
      [] VariantTag(t_3446[key_3444]) = "Leaf"
        -> LET (*
          @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => ({ key_hash: Seq(Int), version: Int } -> { left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }));
        *)
        __QUINT_LAMBDA48(id__3442) == internals_3444
        IN
        __QUINT_LAMBDA48(VariantGetUnsafe("Leaf", t_3446[key_3444]))
  IN
  ApaFoldSet(__QUINT_LAMBDA49, (SetAsFun({})), (DOMAIN t_3446))

(*
  @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), Int) => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
*)
nodesAtVersion(t_3624, version_3624) ==
  [
    nId_3622 \in
      { nId_3616 \in DOMAIN t_3624: nId_3616["version"] = version_3624 } |->
      t_3624[nId_3622]
  ]

(*
  @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), Int) => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
*)
nodesUpToVersion(t_3647, version_3647) ==
  [
    nId_3645 \in
      { nId_3639 \in DOMAIN t_3647: nId_3639["version"] <= version_3647 } |->
      t_3647[nId_3645]
  ]

(*
  @type: ((Set(n)) => n);
*)
getOnlyElement(s_7843) ==
  LET (*
    @type: (() => (Str -> n));
  *)
  hack == SetAsFun({ <<"value", e_7827>>: e_7827 \in s_7843 })
  IN
  IF Cardinality(s_7843) /= 1
  THEN (hack)["error: expected singleton"]
  ELSE (hack)["value"]

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, Int, Int, Seq(Int)) => { nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) });
*)
mark_node_as_orphaned(tree_3278, orphaned_since_version_3278, version_3278, key_hash_3278) ==
  LET (*
    @type: (() => { key_hash: Seq(Int), orphaned_since_version: Int, version: Int });
  *)
  orphan ==
    [orphaned_since_version |-> orphaned_since_version_3278,
      version |-> version_3278,
      key_hash |-> key_hash_3278]
  IN
  [ tree_3278 EXCEPT !["orphans"] = tree_3278["orphans"] \union {(orphan)} ]

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }) => Int);
*)
treeVersion(t_3479) ==
  LET (*
    @type: ((Int, { key_hash: Seq(Int), version: Int }) => Int);
  *)
  __QUINT_LAMBDA51(s_3477, x_3477) ==
    IF x_3477["version"] > s_3477 THEN x_3477["version"] ELSE s_3477
  IN
  ApaFoldSet(__QUINT_LAMBDA51, (-1), {
    nId_3461 \in DOMAIN (t_3479["nodes"]):
      nId_3461["key_hash"] = <<>>
  })

(*
  @type: (((p -> q), p) => Bool);
*)
has(m_1460, key_1460) == key_1460 \in DOMAIN m_1460

(*
  @type: (((r -> s)) => Set(s));
*)
values(m_1657) == { m_1657[k_1655]: k_1655 \in DOMAIN m_1657 }

(*
  @type: ((Set(t)) => Bool);
*)
empty(s_1702) == s_1702 = {}

(*
  @type: ((Seq(u), u, ((u, u) => EQ({ tag: Str }) | GT({ tag: Str }) | LT({ tag: Str }))) => Seq(u));
*)
sortedListInsert(__l_2497, __x_2497, __cmp_2497(_, _)) ==
  LET (*
    @type: (() => { acc: Seq(v), is_inserted: Bool });
  *)
  __init == [is_inserted |-> FALSE, acc |-> <<>>]
  IN
  LET (*
    @type: (() => { acc: Seq(u), is_inserted: Bool });
  *)
  __result ==
    LET (*
      @type: (({ acc: Seq(u), is_inserted: Bool }, u) => { acc: Seq(u), is_inserted: Bool });
    *)
    __QUINT_LAMBDA61(__state_2479, __y_2479) ==
      IF __state_2479["is_inserted"]
      THEN [
        __state_2479 EXCEPT
          !["acc"] = Append(__state_2479["acc"], __y_2479)
      ]
      ELSE CASE VariantTag((__cmp_2497(__x_2497, __y_2479))) = "GT"
          -> LET (*
            @type: (({ tag: Str }) => { acc: Seq(u), is_inserted: Bool });
          *)
          __QUINT_LAMBDA59(id__2473) ==
            [
              __state_2479 EXCEPT
                !["acc"] = Append(__state_2479["acc"], __y_2479)
            ]
          IN
          __QUINT_LAMBDA59(VariantGetUnsafe("GT", (__cmp_2497(__x_2497, __y_2479))))
        [] OTHER
          -> LET (*
            @type: ((w) => { acc: Seq(u), is_inserted: Bool });
          *)
          __QUINT_LAMBDA60(id__2476) ==
            [is_inserted |-> TRUE,
              acc |-> Append((Append(__state_2479["acc"], __x_2497)), __y_2479)]
          IN
          __QUINT_LAMBDA60({})
    IN
    ApaFoldSeqLeft(__QUINT_LAMBDA61, (__init), __l_2497)
  IN
  IF ~((__result)["is_inserted"])
  THEN Append((__result)["acc"], __x_2497)
  ELSE (__result)["acc"]

(*
  @type: ((Seq(x), x, ((x, x) => EQ({ tag: Str }) | GT({ tag: Str }) | LT({ tag: Str }))) => Seq(x));
*)
fancy_sortedListInsert(fancy___l_2497, fancy___x_2497, fancy___cmp_2497(_, _)) ==
  LET (*
    @type: (() => { acc: Seq(y), is_inserted: Bool });
  *)
  fancy___init == [is_inserted |-> FALSE, acc |-> <<>>]
  IN
  LET (*
    @type: (() => { acc: Seq(x), is_inserted: Bool });
  *)
  fancy___result ==
    LET (*
      @type: (({ acc: Seq(x), is_inserted: Bool }, x) => { acc: Seq(x), is_inserted: Bool });
    *)
    __QUINT_LAMBDA64(fancy___state_2479, fancy___y_2479) ==
      IF fancy___state_2479["is_inserted"]
      THEN [
        fancy___state_2479 EXCEPT
          !["acc"] = Append(fancy___state_2479["acc"], fancy___y_2479)
      ]
      ELSE CASE VariantTag((fancy___cmp_2497(fancy___x_2497, fancy___y_2479)))
          = "GT"
          -> LET (*
            @type: (({ tag: Str }) => { acc: Seq(x), is_inserted: Bool });
          *)
          __QUINT_LAMBDA62(fancy___2473) ==
            [
              fancy___state_2479 EXCEPT
                !["acc"] = Append(fancy___state_2479["acc"], fancy___y_2479)
            ]
          IN
          __QUINT_LAMBDA62(VariantGetUnsafe("GT", (fancy___cmp_2497(fancy___x_2497,
          fancy___y_2479))))
        [] OTHER
          -> LET (*
            @type: ((z) => { acc: Seq(x), is_inserted: Bool });
          *)
          __QUINT_LAMBDA63(fancy___2476) ==
            [is_inserted |-> TRUE,
              acc |->
                Append((Append(fancy___state_2479["acc"], fancy___x_2497)), fancy___y_2479)]
          IN
          __QUINT_LAMBDA63({})
    IN
    ApaFoldSeqLeft(__QUINT_LAMBDA64, (fancy___init), fancy___l_2497)
  IN
  IF ~((fancy___result)["is_inserted"])
  THEN Append((fancy___result)["acc"], fancy___x_2497)
  ELSE (fancy___result)["acc"]

(*
  @type: (() => EQ({ tag: Str }) | GT({ tag: Str }) | LT({ tag: Str }));
*)
LT == Variant("LT", [tag |-> "UNIT"])

(*
  @type: (() => EQ({ tag: Str }) | GT({ tag: Str }) | LT({ tag: Str }));
*)
GT == Variant("GT", [tag |-> "UNIT"])

(*
  @type: (() => EQ({ tag: Str }) | GT({ tag: Str }) | LT({ tag: Str }));
*)
EQ == Variant("EQ", [tag |-> "UNIT"])

(*
  @type: ((None({ tag: Str }) | Some(a27)) => a27);
*)
unwrap(value_4076) ==
  CASE VariantTag(value_4076) = "None"
      -> LET (*
        @type: (({ tag: Str }) => a27);
      *)
      __QUINT_LAMBDA66(id__4071) == SetAsFun({})[value_4076]
      IN
      __QUINT_LAMBDA66(VariantGetUnsafe("None", value_4076))
    [] VariantTag(value_4076) = "Some"
      -> LET (*@type: ((a27) => a27); *) __QUINT_LAMBDA67(x_4074) == x_4074 IN
      __QUINT_LAMBDA67(VariantGetUnsafe("Some", value_4076))

(*
  @type: ((Seq(a28)) => Seq(a28));
*)
reverse(l_7793) ==
  LET (*@type: (() => Int); *) len == Len(l_7793) IN
  LET (*
    @type: ((Seq(a28), Int) => Seq(a28));
  *)
  __QUINT_LAMBDA68(acc_7790, i_7790) ==
    [ acc_7790 EXCEPT ![i_7790 + 1] = l_7793[((len - i_7790) - 1 + 1)] ]
  IN
  LET (*@type: (() => Set(Int)); *) __quint_var16 == DOMAIN l_7793 IN
  ApaFoldSet(__QUINT_LAMBDA68, l_7793, (IF __quint_var16 = {}
  THEN {}
  ELSE (__quint_var16 \union {0}) \ {(Len(l_7793))}))

(*
  @type: (({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) => Exist({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) | NonExist({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) }));
*)
Exist(__ExistParam_7546) == Variant("Exist", __ExistParam_7546)

(*
  @type: (({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) }) => Exist({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) | NonExist({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) }));
*)
NonExist(__NonExistParam_7552) == Variant("NonExist", __NonExistParam_7552)

(*
  @type: (() => Int);
*)
keylength == 3

(*
  @type: ((Seq(a40), Seq(a40)) => Bool);
*)
prefix_of(l1_4235, l2_4235) ==
  Len(l1_4235) <= Len(l2_4235)
    /\ (\A i_4232 \in LET (*
      @type: (() => Set(Int));
    *)
    __quint_var25 == DOMAIN l1_4235
    IN
    IF __quint_var25 = {}
    THEN {}
    ELSE (__quint_var25 \union {0}) \ {(Len(l1_4235))}:
      l1_4235[(i_4232 + 1)] = l2_4235[(i_4232 + 1)])

(*
  @type: ((Seq(a42)) => a42);
*)
last(v_1948) == v_1948[(Len(v_1948) - 1 + 1)]

(*
  @type: ((a44, a45) => a45);
*)
q_debug(s_7892, a_7892) == a_7892

(*
  @type: (((a29 -> a35), a29) => Bool);
*)
fancy_has(fancy_m_1460, fancy_key_1460) ==
  fancy_key_1460 \in DOMAIN fancy_m_1460

(*
  @type: (() => Seq(Int));
*)
fancy_ROOT_BITS == <<>>

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, Int, Int, Seq(Int)) => { nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) });
*)
fancy_mark_node_as_orphaned(fancy_tree_3278, fancy_orphaned_since_version_3278, fancy_version_3278,
fancy_key_hash_3278) ==
  LET (*
    @type: (() => { key_hash: Seq(Int), orphaned_since_version: Int, version: Int });
  *)
  fancy_orphan ==
    [orphaned_since_version |-> fancy_orphaned_since_version_3278,
      version |-> fancy_version_3278,
      key_hash |-> fancy_key_hash_3278]
  IN
  [
    fancy_tree_3278 EXCEPT
      !["orphans"] = fancy_tree_3278["orphans"] \union {(fancy_orphan)}
  ]

(*
  @type: (() => EQ({ tag: Str }) | GT({ tag: Str }) | LT({ tag: Str }));
*)
fancy_LT == Variant("LT", [tag |-> "UNIT"])

(*
  @type: (() => EQ({ tag: Str }) | GT({ tag: Str }) | LT({ tag: Str }));
*)
fancy_GT == Variant("GT", [tag |-> "UNIT"])

(*
  @type: (() => EQ({ tag: Str }) | GT({ tag: Str }) | LT({ tag: Str }));
*)
fancy_EQ == Variant("EQ", [tag |-> "UNIT"])

(*
  @type: ((Seq(a41), Seq(a41)) => Bool);
*)
fancy_prefix_of(fancy_l1_4235, fancy_l2_4235) ==
  Len(fancy_l1_4235) <= Len(fancy_l2_4235)
    /\ (\A fancy_i_4232 \in LET (*
      @type: (() => Set(Int));
    *)
    __quint_var33 == DOMAIN fancy_l1_4235
    IN
    IF __quint_var33 = {}
    THEN {}
    ELSE (__quint_var33 \union {0}) \ {(Len(fancy_l1_4235))}:
      fancy_l1_4235[(fancy_i_4232 + 1)] = fancy_l2_4235[(fancy_i_4232 + 1)])

(*
  @type: ((Set(g), ((g) => None({ tag: Str }) | Some(h))) => Set(h));
*)
fancy_filterMap(fancy_s_4129, fancy_f_4129(_)) ==
  LET (*
    @type: ((Set(h), g) => Set(h));
  *)
  __QUINT_LAMBDA174(fancy_acc_4127, fancy_e_4127) ==
    CASE VariantTag((fancy_f_4129(fancy_e_4127))) = "Some"
        -> LET (*
          @type: ((h) => Set(h));
        *)
        __QUINT_LAMBDA172(fancy_x_4122) == fancy_acc_4127 \union {fancy_x_4122}
        IN
        __QUINT_LAMBDA172(VariantGetUnsafe("Some", (fancy_f_4129(fancy_e_4127))))
      [] VariantTag((fancy_f_4129(fancy_e_4127))) = "None"
        -> LET (*
          @type: (({ tag: Str }) => Set(h));
        *)
        __QUINT_LAMBDA173(fancy___4125) == fancy_acc_4127
        IN
        __QUINT_LAMBDA173(VariantGetUnsafe("None", (fancy_f_4129(fancy_e_4127))))
  IN
  ApaFoldSet(__QUINT_LAMBDA174, {}, fancy_s_4129)

(*
  @type: ((None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
*)
fancy_Unchanged(fancy___UnchangedParam_3894) ==
  Variant("Unchanged", fancy___UnchangedParam_3894)

(*
  @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }));
*)
fancy_Leaf(fancy___LeafParam_3881) == Variant("Leaf", fancy___LeafParam_3881)

(*
  @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
*)
fancy_Updated(fancy___UpdatedParam_3900) ==
  Variant("Updated", fancy___UpdatedParam_3900)

(*
  @type: (() => Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
*)
fancy_Deleted == Variant("Deleted", [tag |-> "UNIT"])

(*
  @type: (() => Int);
*)
fancy_MAX_HASH_LENGTH == 3

(*
  @type: ((Seq(a56), Seq(a56)) => Bool);
*)
fancy_isPrefixOf(fancy_l1_2090, fancy_l2_2090) ==
  IF Len(fancy_l1_2090) > Len(fancy_l2_2090)
  THEN FALSE
  ELSE \A fancy_i_2087 \in LET (*
    @type: (() => Set(Int));
  *)
  __quint_var34 == DOMAIN fancy_l1_2090
  IN
  IF __quint_var34 = {}
  THEN {}
  ELSE (__quint_var34 \union {0}) \ {(Len(fancy_l1_2090))}:
    fancy_l1_2090[(fancy_i_2087 + 1)] = fancy_l2_2090[(fancy_i_2087 + 1)]

(*
  @type: ((None({ tag: Str }) | Some(a30)) => a30);
*)
fancy_unwrap(fancy_value_4076) ==
  CASE VariantTag(fancy_value_4076) = "None"
      -> LET (*
        @type: (({ tag: Str }) => a30);
      *)
      __QUINT_LAMBDA181(fancy___4071) == SetAsFun({})[fancy_value_4076]
      IN
      __QUINT_LAMBDA181(VariantGetUnsafe("None", fancy_value_4076))
    [] VariantTag(fancy_value_4076) = "Some"
      -> LET (*
        @type: ((a30) => a30);
      *)
      __QUINT_LAMBDA182(fancy_x_4074) == fancy_x_4074
      IN
      __QUINT_LAMBDA182(VariantGetUnsafe("Some", fancy_value_4076))

(*
  @type: ((Set({ key_hash: Seq(Int), a57 }), Seq(Int)) => <<Set({ key_hash: Seq(Int), a57 }), Set({ key_hash: Seq(Int), a57 })>>);
*)
fancy_partition_batch(fancy_batch_6720, fancy_bits_6720) ==
  LET (*
    @type: ((<<Set({ key_hash: Seq(Int), a57 }), Set({ key_hash: Seq(Int), a57 })>>, { key_hash: Seq(Int), a57 }) => <<Set({ key_hash: Seq(Int), a57 }), Set({ key_hash: Seq(Int), a57 })>>);
  *)
  __QUINT_LAMBDA183(fancy_acc_6718, fancy_op_6718) ==
    IF fancy_op_6718["key_hash"][(Len(fancy_bits_6720) + 1)] = 0
    THEN <<(fancy_acc_6718[1] \union {fancy_op_6718}), fancy_acc_6718[2]>>
    ELSE <<fancy_acc_6718[1], (fancy_acc_6718[2] \union {fancy_op_6718})>>
  IN
  ApaFoldSet(__QUINT_LAMBDA183, <<{}, {}>>, fancy_batch_6720)

(*
  @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }));
*)
fancy_Internal(fancy___InternalParam_3875) ==
  Variant("Internal", fancy___InternalParam_3875)

(*
  @type: ((Set(a64), Int) => Set(Seq(a64)));
*)
allListsUpTo(s_7887, l_7887) ==
  LET (*
    @type: ((Set(Seq(a64)), Int) => Set(Seq(a64)));
  *)
  __QUINT_LAMBDA201(acc_7885, i_7885) ==
    LET (*
      @type: (() => Set((Int -> a64)));
    *)
    kms == [(0 .. i_7885 - 1) -> s_7887]
    IN
    LET (*
      @type: (() => Set(Seq(a64)));
    *)
    lists ==
      {
        LET (*
          @type: ((Seq(a64), Int) => Seq(a64));
        *)
        __QUINT_LAMBDA200(acc_7875, i_7875) == Append(acc_7875, km_7877[i_7875])
        IN
        ApaFoldSeqLeft(__QUINT_LAMBDA200, <<>>, (LET (*
          @type: ((Int) => Int);
        *)
        __QUINT_LAMBDA199(__quint_var35) == (0 + __quint_var35) - 1
        IN
        MkSeq((i_7885 - 0), __QUINT_LAMBDA199))):
          km_7877 \in kms
      }
    IN
    acc_7885 \union lists
  IN
  ApaFoldSet(__QUINT_LAMBDA201, {<<>>}, (1 .. l_7887))

(*
  @type: ((Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Bool);
*)
fancy_is_updated_or_deleted(fancy_outcome_4299) ==
  CASE VariantTag(fancy_outcome_4299) = "Updated"
      -> LET (*
        @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => Bool);
      *)
      __QUINT_LAMBDA217(fancy___4291) == TRUE
      IN
      __QUINT_LAMBDA217(VariantGetUnsafe("Updated", fancy_outcome_4299))
    [] VariantTag(fancy_outcome_4299) = "Deleted"
      -> LET (*
        @type: (({ tag: Str }) => Bool);
      *)
      __QUINT_LAMBDA218(fancy___4294) == TRUE
      IN
      __QUINT_LAMBDA218(VariantGetUnsafe("Deleted", fancy_outcome_4299))
    [] OTHER
      -> LET (*
        @type: ((a71) => Bool);
      *)
      __QUINT_LAMBDA219(fancy___4297) == FALSE
      IN
      __QUINT_LAMBDA219({})

(*
  @type: ((Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Bool);
*)
fancy_is_unchanged(fancy_outcome_4314) ==
  CASE VariantTag(fancy_outcome_4314) = "Unchanged"
      -> LET (*
        @type: ((None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Bool);
      *)
      __QUINT_LAMBDA220(fancy___4309) == TRUE
      IN
      __QUINT_LAMBDA220(VariantGetUnsafe("Unchanged", fancy_outcome_4314))
    [] OTHER
      -> LET (*
        @type: ((a72) => Bool);
      *)
      __QUINT_LAMBDA221(fancy___4312) == FALSE
      IN
      __QUINT_LAMBDA221({})

(*
  @type: ((Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Bool);
*)
fancy_updated_to_leaf(fancy_outcome_4371) ==
  CASE VariantTag(fancy_outcome_4371) = "Updated"
      -> LET (*
        @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => Bool);
      *)
      __QUINT_LAMBDA224(fancy_node_4366) ==
        CASE VariantTag(fancy_node_4366) = "Leaf"
            -> LET (*
              @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => Bool);
            *)
            __QUINT_LAMBDA222(fancy___4358) == TRUE
            IN
            __QUINT_LAMBDA222(VariantGetUnsafe("Leaf", fancy_node_4366))
          [] OTHER
            -> LET (*
              @type: ((a73) => Bool);
            *)
            __QUINT_LAMBDA223(fancy___4361) == FALSE
            IN
            __QUINT_LAMBDA223({})
      IN
      __QUINT_LAMBDA224(VariantGetUnsafe("Updated", fancy_outcome_4371))
    [] OTHER
      -> LET (*
        @type: ((a74) => Bool);
      *)
      __QUINT_LAMBDA225(fancy___4369) == FALSE
      IN
      __QUINT_LAMBDA225({})

(*
  @type: ((Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Bool);
*)
fancy_unchanged_leaf(fancy_outcome_4347) ==
  CASE VariantTag(fancy_outcome_4347) = "Unchanged"
      -> LET (*
        @type: ((None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Bool);
      *)
      __QUINT_LAMBDA230(fancy_optional_4342) ==
        CASE VariantTag(fancy_optional_4342) = "Some"
            -> LET (*
              @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => Bool);
            *)
            __QUINT_LAMBDA228(fancy_node_4334) ==
              CASE VariantTag(fancy_node_4334) = "Leaf"
                  -> LET (*
                    @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => Bool);
                  *)
                  __QUINT_LAMBDA226(fancy___4326) == TRUE
                  IN
                  __QUINT_LAMBDA226(VariantGetUnsafe("Leaf", fancy_node_4334))
                [] OTHER
                  -> LET (*
                    @type: ((a75) => Bool);
                  *)
                  __QUINT_LAMBDA227(fancy___4329) == FALSE
                  IN
                  __QUINT_LAMBDA227({})
            IN
            __QUINT_LAMBDA228(VariantGetUnsafe("Some", fancy_optional_4342))
          [] OTHER
            -> LET (*
              @type: ((a76) => Bool);
            *)
            __QUINT_LAMBDA229(fancy___4337) == FALSE
            IN
            __QUINT_LAMBDA229({})
      IN
      __QUINT_LAMBDA230(VariantGetUnsafe("Unchanged", fancy_outcome_4347))
    [] OTHER
      -> LET (*
        @type: ((a77) => Bool);
      *)
      __QUINT_LAMBDA231(fancy___4345) == FALSE
      IN
      __QUINT_LAMBDA231({})

(*
  @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>)) => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
*)
fancy_add_nodes(fancy_nodes_3817, fancy_new_nodes_3817) ==
  LET (*
    @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), <<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>) => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
  *)
  __QUINT_LAMBDA249(fancy_nodes_3815, fancy_new_node_3815) ==
    LET (*
      @type: (() => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
    *)
    __quint_var43 == fancy_nodes_3815
    IN
    LET (*
      @type: (() => Set({ key_hash: Seq(Int), version: Int }));
    *)
    __quint_var44 == DOMAIN __quint_var43
    IN
    [
      __quint_var45 \in {fancy_new_node_3815[1]} \union __quint_var44 |->
        IF __quint_var45 = fancy_new_node_3815[1]
        THEN fancy_new_node_3815[2]
        ELSE (__quint_var43)[__quint_var45]
    ]
  IN
  ApaFoldSet(__QUINT_LAMBDA249, fancy_nodes_3817, fancy_new_nodes_3817)

(*
  @type: ((Seq(a31)) => Seq(a31));
*)
fancy_reverse(fancy_l_7793) ==
  LET (*@type: (() => Int); *) fancy_len == Len(fancy_l_7793) IN
  LET (*
    @type: ((Seq(a31), Int) => Seq(a31));
  *)
  __QUINT_LAMBDA255(fancy_acc_7790, fancy_i_7790) ==
    [
      fancy_acc_7790 EXCEPT
        ![fancy_i_7790 + 1] = fancy_l_7793[((fancy_len - fancy_i_7790) - 1 + 1)]
    ]
  IN
  LET (*@type: (() => Set(Int)); *) __quint_var52 == DOMAIN fancy_l_7793 IN
  ApaFoldSet(__QUINT_LAMBDA255, fancy_l_7793, (IF __quint_var52 = {}
  THEN {}
  ELSE (__quint_var52 \union {0}) \ {(Len(fancy_l_7793))}))

(*
  @type: ((Set(a67), Int) => Set(Seq(a67)));
*)
fancy_allListsUpTo(fancy_s_7887, fancy_l_7887) ==
  LET (*
    @type: ((Set(Seq(a67)), Int) => Set(Seq(a67)));
  *)
  __QUINT_LAMBDA259(fancy_acc_7885, fancy_i_7885) ==
    LET (*
      @type: (() => Set((Int -> a67)));
    *)
    fancy_kms == [(0 .. fancy_i_7885 - 1) -> fancy_s_7887]
    IN
    LET (*
      @type: (() => Set(Seq(a67)));
    *)
    fancy_lists ==
      {
        LET (*
          @type: ((Seq(a67), Int) => Seq(a67));
        *)
        __QUINT_LAMBDA258(fancy_acc_7875, fancy_i_7875) ==
          Append(fancy_acc_7875, fancy_km_7877[fancy_i_7875])
        IN
        ApaFoldSeqLeft(__QUINT_LAMBDA258, <<>>, (LET (*
          @type: ((Int) => Int);
        *)
        __QUINT_LAMBDA257(__quint_var53) == (0 + __quint_var53) - 1
        IN
        MkSeq((fancy_i_7885 - 0), __QUINT_LAMBDA257))):
          fancy_km_7877 \in fancy_kms
      }
    IN
    fancy_acc_7885 \union fancy_lists
  IN
  ApaFoldSet(__QUINT_LAMBDA259, {<<>>}, (1 .. fancy_l_7887))

(*
  @type: ((Set(a52)) => a52);
*)
fancy_getOnlyElement(fancy_s_7843) ==
  LET (*
    @type: (() => (Str -> a52));
  *)
  fancy_hack ==
    SetAsFun({ <<"value", fancy_e_7827>>: fancy_e_7827 \in fancy_s_7843 })
  IN
  IF Cardinality(fancy_s_7843) /= 1
  THEN (fancy_hack)["error: expected singleton"]
  ELSE (fancy_hack)["value"]

(*
  @type: (() => Set(Int));
*)
versionsToCheck == {version}

(*
  @type: (((Seq(Int) -> None({ tag: Str }) | Some(Delete({ tag: Str }) | Insert(Seq(Int))))) => Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }));
*)
to_operations(nondet_value_114) ==
  LET (*
    @type: ((<<Seq(Int), None({ tag: Str }) | Some(Delete({ tag: Str }) | Insert(Seq(Int)))>>) => None({ tag: Str }) | Some({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }));
  *)
  __QUINT_LAMBDA5(quintTupledLambdaParam103_112) ==
    LET (*
      @type: (() => None({ tag: Str }) | Some(Delete({ tag: Str }) | Insert(Seq(Int))));
    *)
    maybe_op == quintTupledLambdaParam103_112[2]
    IN
    LET (*
      @type: (() => Seq(Int));
    *)
    key_hash == quintTupledLambdaParam103_112[1]
    IN
    CASE VariantTag((maybe_op)) = "Some"
        -> LET (*
          @type: ((Delete({ tag: Str }) | Insert(Seq(Int))) => None({ tag: Str }) | Some({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }));
        *)
        __QUINT_LAMBDA3(op_98) == Some([key_hash |-> key_hash, op |-> op_98])
        IN
        __QUINT_LAMBDA3(VariantGetUnsafe("Some", (maybe_op)))
      [] VariantTag((maybe_op)) = "None"
        -> LET (*
          @type: (({ tag: Str }) => None({ tag: Str }) | Some({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }));
        *)
        __QUINT_LAMBDA4(id__101) == None
        IN
        __QUINT_LAMBDA4(VariantGetUnsafe("None", (maybe_op)))
  IN
  filterMap((mapToTuples(nondet_value_114)), __QUINT_LAMBDA5)

(*
  @type: (() => Set(None({ tag: Str }) | Some(Delete({ tag: Str }) | Insert(Seq(Int)))));
*)
INIT_VALUES == { (Some((Insert(<<1>>)))), (None) }

(*
  @type: (() => Set(None({ tag: Str }) | Some(Delete({ tag: Str }) | Insert(Seq(Int)))));
*)
VALUES ==
  { (Some((Insert(<<1>>)))), (Some((Insert(<<2>>)))), (Some((Delete))), (None) }

(*
  @type: ((Seq(Int)) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
raw(bytes_2969) == SetAsFun({<<<<0>>, (Raw(bytes_2969))>>})

(*
  @type: (((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))) => Bool);
*)
isRaw(term_2986) ==
  Cardinality((DOMAIN term_2986)) = 1 /\ term_2986[<<0>>] /= Hash

(*
  @type: (((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
termHash(term_3023) ==
  LET (*
    @type: (() => Set(Seq(Int)));
  *)
  paths == {<<0>>} \union { <<0>> \o p_3001: p_3001 \in DOMAIN term_3023 }
  IN
  [
    p_3020 \in paths |->
      IF p_3020 = <<0>>
      THEN Hash
      ELSE term_3023[(SubSeq(p_3020, (1 + 1), (Len(p_3020))))]
  ]

(*
  @type: ((Seq(Int)) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
fancy_raw(fancy_bytes_2969) ==
  SetAsFun({<<<<0>>, (fancy_Raw(fancy_bytes_2969))>>})

(*
  @type: (((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))) => Bool);
*)
fancy_isRaw(fancy_term_2986) ==
  Cardinality((DOMAIN fancy_term_2986)) = 1
    /\ fancy_term_2986[<<0>>] /= fancy_Hash

(*
  @type: (((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
fancy_termHash(fancy_term_3023) ==
  LET (*
    @type: (() => Set(Seq(Int)));
  *)
  fancy_paths ==
    {<<0>>}
      \union { <<0>> \o fancy_p_3001: fancy_p_3001 \in DOMAIN fancy_term_3023 }
  IN
  [
    fancy_p_3020 \in fancy_paths |->
      IF fancy_p_3020 = <<0>>
      THEN fancy_Hash
      ELSE fancy_term_3023[(SubSeq(fancy_p_3020, (1 + 1), (Len(fancy_p_3020))))]
  ]

(*
  @type: (({ key_hash: Seq(Int), version: Int }) => Bool);
*)
isRoot(key_3315) == key_3315["key_hash"] = ROOT_BITS

(*
  @type: (() => Set(Seq(Int)));
*)
all_key_hashes ==
  LET (*
    @type: (() => Set((Int -> Int)));
  *)
  kms == [(0 .. MAX_HASH_LENGTH - 1) -> { 0, 1 }]
  IN
  {
    LET (*
      @type: ((Seq(Int), Int) => Seq(Int));
    *)
    __QUINT_LAMBDA36(acc_3779, i_3779) == Append(acc_3779, km_3781[i_3779])
    IN
    ApaFoldSeqLeft(__QUINT_LAMBDA36, <<>>, (LET (*
      @type: ((Int) => Int);
    *)
    __QUINT_LAMBDA35(__quint_var6) == (0 + __quint_var6) - 1
    IN
    MkSeq((MAX_HASH_LENGTH - 0), __QUINT_LAMBDA35))):
      km_3781 \in kms
  }

(*
  @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), Seq(Int)) => None({ tag: Str }) | Some({ key_hash: Seq(Int), version: Int }));
*)
mostRecentNodeId(nodes_3601, key_3601) ==
  LET (*
    @type: (() => { key_hash: Seq(Int), version: Int });
  *)
  default == [version |-> -1, key_hash |-> key_3601]
  IN
  LET (*
    @type: (() => { key_hash: Seq(Int), version: Int });
  *)
  result ==
    LET (*
      @type: (({ key_hash: Seq(Int), version: Int }, { key_hash: Seq(Int), version: Int }) => { key_hash: Seq(Int), version: Int });
    *)
    __QUINT_LAMBDA50(acc_3589, e_3589) ==
      IF e_3589["version"] > acc_3589["version"] THEN e_3589 ELSE acc_3589
    IN
    ApaFoldSet(__QUINT_LAMBDA50, (default), {
      nId_3574 \in DOMAIN nodes_3601:
        nId_3574["key_hash"] = key_3601
    })
  IN
  IF result = default THEN None ELSE Some((result))

(*
  @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), Seq(Int)) => Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }));
*)
findNode(t_3733, key_hash_3733) ==
  LET (*
    @type: (() => { key_hash: Seq(Int), version: Int });
  *)
  nodeId ==
    getOnlyElement({
      n_3725 \in DOMAIN t_3733:
        n_3725["key_hash"] = key_hash_3733
    })
  IN
  t_3733[(nodeId)]

(*
  @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => Bool);
*)
isLeaf(n_3754) == ~(isInternal(n_3754))

(*
  @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Set(Seq(Int)));
*)
childrenPrefixes(nodes_3550) ==
  LET (*
    @type: (() => Set(Seq(Int)));
  *)
  allChildren ==
    LET (*
      @type: ((Set(Seq(Int)), { key_hash: Seq(Int), version: Int }) => Set(Seq(Int)));
    *)
    __QUINT_LAMBDA54(s_3536, x_3536) ==
      CASE VariantTag(nodes_3550[x_3536]) = "Internal"
          -> LET (*
            @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => Set(Seq(Int)));
          *)
          __QUINT_LAMBDA52(n_3531) ==
            LET (*
              @type: (() => Set(Seq(Int)));
            *)
            lc ==
              IF n_3531["left_child"] /= None
              THEN {(Append(x_3536["key_hash"], 0))}
              ELSE {}
            IN
            LET (*
              @type: (() => Set(Seq(Int)));
            *)
            rc ==
              IF n_3531["right_child"] /= None
              THEN {(Append(x_3536["key_hash"], 1))}
              ELSE {}
            IN
            (s_3536 \union lc) \union rc
          IN
          __QUINT_LAMBDA52(VariantGetUnsafe("Internal", nodes_3550[x_3536]))
        [] VariantTag(nodes_3550[x_3536]) = "Leaf"
          -> LET (*
            @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => Set(Seq(Int)));
          *)
          __QUINT_LAMBDA53(id__3534) == s_3536
          IN
          __QUINT_LAMBDA53(VariantGetUnsafe("Leaf", nodes_3550[x_3536]))
    IN
    ApaFoldSet(__QUINT_LAMBDA54, {}, (DOMAIN nodes_3550))
  IN
  allChildren \ { nId_3546["key_hash"]: nId_3546 \in DOMAIN nodes_3550 }

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, Int) => { nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) });
*)
prune(tree_3377, up_to_version_3377) ==
  LET (*
    @type: (() => Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }));
  *)
  orphans_to_be_removed ==
    {
      orphan_3331 \in tree_3377["orphans"]:
        orphan_3331["orphaned_since_version"] <= up_to_version_3377
    }
  IN
  LET (*
    @type: (() => Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }));
  *)
  prunned_orphans ==
    {
      orphan_3343 \in tree_3377["orphans"]:
        orphan_3343["orphaned_since_version"] > up_to_version_3377
    }
  IN
  LET (*
    @type: (() => Set({ key_hash: Seq(Int), version: Int }));
  *)
  keys_of_non_orphanes ==
    {
      nodeId_3355 \in DOMAIN (tree_3377["nodes"]):
        ~(is_node_orphaned(nodeId_3355, (orphans_to_be_removed)))
    }
  IN
  LET (*
    @type: (() => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
  *)
  updated_nodes ==
    [ x_3365 \in keys_of_non_orphanes |-> tree_3377["nodes"][x_3365] ]
  IN
  [nodes |-> updated_nodes, orphans |-> prunned_orphans]

(*
  @type: ((Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }), { nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }) => Bool);
*)
assign_result(ops_182, new_tree_182) ==
  tree' := new_tree_182
    /\ version' := (version + 1)
    /\ smallest_unpruned_version' := smallest_unpruned_version
    /\ ops_history' := (Append(ops_history, ops_182))

(*
  @type: (() => Set(Int));
*)
activeTreeVersions == smallest_unpruned_version .. treeVersion(tree)

(*
  @type: (() => Bool);
*)
versionInv ==
  LET (*
    @type: ((Seq(Int)) => Set(Seq(Int)));
  *)
  allPrefixes(l_494) ==
    { SubSeq(l_494, (0 + 1), i_492): i_492 \in 0 .. Len(l_494) }
  IN
  \A a_528 \in DOMAIN (tree["nodes"]):
    \A p_526 \in allPrefixes(a_528["key_hash"]):
      \E b_524 \in DOMAIN (tree["nodes"]):
        p_526 = b_524["key_hash"] /\ b_524["version"] >= a_528["version"]

(*
  @type: ((Set(a26), ((a26, a26) => EQ({ tag: Str }) | GT({ tag: Str }) | LT({ tag: Str }))) => Seq(a26));
*)
toList(__set_2557, __cmp_2557(_, _)) ==
  LET (*
    @type: ((Seq(a26), a26) => Seq(a26));
  *)
  __QUINT_LAMBDA65(__l_2555, __e_2555) ==
    sortedListInsert(__l_2555, __e_2555, __cmp_2557)
  IN
  ApaFoldSet(__QUINT_LAMBDA65, <<>>, __set_2557)

(*
  @type: ((Seq(Int), Seq(Int)) => EQ({ tag: Str }) | GT({ tag: Str }) | LT({ tag: Str }));
*)
listCompare(__a_2362, __b_2362) ==
  IF Len(__a_2362) < Len(__b_2362)
  THEN LT
  ELSE IF Len(__a_2362) > Len(__b_2362) THEN GT ELSE EQ

(*
  @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), Seq(Int)) => None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) }));
*)
leftNeighbor(t_5449, k_5449) ==
  LET (*
    @type: (() => Set(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
  *)
  smallerKeyNodes ==
    {
      n_5393 \in values(t_5449):
        CASE VariantTag(n_5393) = "Leaf"
            -> LET (*
              @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => Bool);
            *)
            __QUINT_LAMBDA76(l_5388) == less_than(l_5388["key_hash"], k_5449)
            IN
            __QUINT_LAMBDA76(VariantGetUnsafe("Leaf", n_5393))
          [] VariantTag(n_5393) = "Internal"
            -> LET (*
              @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => Bool);
            *)
            __QUINT_LAMBDA77(id__5391) == FALSE
            IN
            __QUINT_LAMBDA77(VariantGetUnsafe("Internal", n_5393))
    }
  IN
  IF empty((smallerKeyNodes))
  THEN None
  ELSE LET (*
    @type: (() => { key_hash: Seq(Int), value_hash: Seq(Int) });
  *)
  someLeaf ==
    LET (*
      @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => { key_hash: Seq(Int), value_hash: Seq(Int) });
    *)
    __QUINT_LAMBDA80(s_5417, x_5417) ==
      CASE VariantTag(x_5417) = "Leaf"
          -> LET (*
            @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => { key_hash: Seq(Int), value_hash: Seq(Int) });
          *)
          __QUINT_LAMBDA78(l_5412) == l_5412
          IN
          __QUINT_LAMBDA78(VariantGetUnsafe("Leaf", x_5417))
        [] VariantTag(x_5417) = "Internal"
          -> LET (*
            @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => { key_hash: Seq(Int), value_hash: Seq(Int) });
          *)
          __QUINT_LAMBDA79(id__5415) == s_5417
          IN
          __QUINT_LAMBDA79(VariantGetUnsafe("Internal", x_5417))
    IN
    ApaFoldSet(__QUINT_LAMBDA80, [key_hash |-> <<>>, value_hash |-> <<>>], (smallerKeyNodes))
  IN
  Some(LET (*
    @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => { key_hash: Seq(Int), value_hash: Seq(Int) });
  *)
  __QUINT_LAMBDA83(s_5443, x_5443) ==
    CASE VariantTag(x_5443) = "Leaf"
        -> LET (*
          @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => { key_hash: Seq(Int), value_hash: Seq(Int) });
        *)
        __QUINT_LAMBDA81(l_5438) ==
          IF less_than(s_5443["key_hash"], l_5438["key_hash"])
          THEN l_5438
          ELSE s_5443
        IN
        __QUINT_LAMBDA81(VariantGetUnsafe("Leaf", x_5443))
      [] VariantTag(x_5443) = "Internal"
        -> LET (*
          @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => { key_hash: Seq(Int), value_hash: Seq(Int) });
        *)
        __QUINT_LAMBDA82(id__5441) == s_5443
        IN
        __QUINT_LAMBDA82(VariantGetUnsafe("Internal", x_5443))
  IN
  ApaFoldSet(__QUINT_LAMBDA83, (someLeaf), (smallerKeyNodes)))

(*
  @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), Seq(Int)) => None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) }));
*)
rightNeighbor(t_5531, k_5531) ==
  LET (*
    @type: (() => Set(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
  *)
  largerKeyNodes ==
    {
      n_5475 \in values(t_5531):
        CASE VariantTag(n_5475) = "Leaf"
            -> LET (*
              @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => Bool);
            *)
            __QUINT_LAMBDA84(l_5470) == less_than(k_5531, l_5470["key_hash"])
            IN
            __QUINT_LAMBDA84(VariantGetUnsafe("Leaf", n_5475))
          [] VariantTag(n_5475) = "Internal"
            -> LET (*
              @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => Bool);
            *)
            __QUINT_LAMBDA85(id__5473) == FALSE
            IN
            __QUINT_LAMBDA85(VariantGetUnsafe("Internal", n_5475))
    }
  IN
  IF empty((largerKeyNodes))
  THEN None
  ELSE LET (*
    @type: (() => { key_hash: Seq(Int), value_hash: Seq(Int) });
  *)
  someLeaf ==
    LET (*
      @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => { key_hash: Seq(Int), value_hash: Seq(Int) });
    *)
    __QUINT_LAMBDA88(s_5499, x_5499) ==
      CASE VariantTag(x_5499) = "Leaf"
          -> LET (*
            @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => { key_hash: Seq(Int), value_hash: Seq(Int) });
          *)
          __QUINT_LAMBDA86(l_5494) == l_5494
          IN
          __QUINT_LAMBDA86(VariantGetUnsafe("Leaf", x_5499))
        [] VariantTag(x_5499) = "Internal"
          -> LET (*
            @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => { key_hash: Seq(Int), value_hash: Seq(Int) });
          *)
          __QUINT_LAMBDA87(id__5497) == s_5499
          IN
          __QUINT_LAMBDA87(VariantGetUnsafe("Internal", x_5499))
    IN
    ApaFoldSet(__QUINT_LAMBDA88, [key_hash |-> <<>>, value_hash |-> <<>>], (largerKeyNodes))
  IN
  Some(LET (*
    @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => { key_hash: Seq(Int), value_hash: Seq(Int) });
  *)
  __QUINT_LAMBDA91(s_5525, x_5525) ==
    CASE VariantTag(x_5525) = "Leaf"
        -> LET (*
          @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => { key_hash: Seq(Int), value_hash: Seq(Int) });
        *)
        __QUINT_LAMBDA89(l_5520) ==
          IF less_than(l_5520["key_hash"], s_5525["key_hash"])
          THEN l_5520
          ELSE s_5525
        IN
        __QUINT_LAMBDA89(VariantGetUnsafe("Leaf", x_5525))
      [] VariantTag(x_5525) = "Internal"
        -> LET (*
          @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => { key_hash: Seq(Int), value_hash: Seq(Int) });
        *)
        __QUINT_LAMBDA90(id__5523) == s_5525
        IN
        __QUINT_LAMBDA90(VariantGetUnsafe("Internal", x_5525))
  IN
  ApaFoldSet(__QUINT_LAMBDA91, (someLeaf), (largerKeyNodes)))

(*
  @type: ((Seq(a36), ((a36) => Bool)) => None({ tag: Str }) | Some(a29));
*)
findFirst(l_2194, f_2194(_)) ==
  LET (*
    @type: ((None({ tag: Str }) | Some(a36), a36) => None({ tag: Str }) | Some(a29));
  *)
  __QUINT_LAMBDA111(a_2192, i_2192) ==
    IF a_2192 = None /\ f_2194(i_2192) THEN Some(i_2192) ELSE a_2192
  IN
  ApaFoldSeqLeft(__QUINT_LAMBDA111, (None), l_2194)

(*
  @type: (({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, { max_prefix: Int, min_prefix: Int, suffix: Int }) => Bool);
*)
has_padding(op_4827, pad_4827) ==
  termLen(op_4827["prefix"]) >= pad_4827["min_prefix"]
    /\ termLen(op_4827["prefix"]) <= pad_4827["max_prefix"]
    /\ termLen(op_4827["suffix"]) = pad_4827["suffix"]

(*
  @type: ((Seq(a38), ((a38) => Bool)) => None({ tag: Str }) | Some(a29));
*)
find_first(l_2214, f_2214(_)) ==
  LET (*
    @type: ((None({ tag: Str }) | Some(a38), a38) => None({ tag: Str }) | Some(a29));
  *)
  __QUINT_LAMBDA115(a_2212, i_2212) ==
    IF a_2212 = None /\ f_2214(i_2212) THEN Some(i_2212) ELSE a_2212
  IN
  ApaFoldSeqLeft(__QUINT_LAMBDA115, (None), l_2214)

(*
  @type: ((Int, Int) => None({ tag: Str }) | Some(Int));
*)
checked_sub(a_4389, b_4389) ==
  IF b_4389 > a_4389 THEN None ELSE Some((a_4389 - b_4389))

(*
  @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }, { key_hash: Seq(Int), value_hash: Seq(Int) }) => Seq(Int));
*)
commonPrefix(a_4208, b_4208) ==
  LET (*
    @type: (() => Seq(Int));
  *)
  indList ==
    LET (*
      @type: ((Int) => Int);
    *)
    __QUINT_LAMBDA148(__quint_var23) == (1 + __quint_var23) - 1
    IN
    MkSeq(((keylength + 1) - 1), __QUINT_LAMBDA148)
  IN
  LET (*
    @type: ((Seq(Int), Int) => Seq(Int));
  *)
  __QUINT_LAMBDA149(s_4205, x_4205) ==
    IF SubSeq(a_4208["key_hash"], (0 + 1), x_4205)
      = SubSeq(b_4208["key_hash"], (0 + 1), x_4205)
    THEN SubSeq(b_4208["key_hash"], (0 + 1), x_4205)
    ELSE s_4205
  IN
  ApaFoldSeqLeft(__QUINT_LAMBDA149, <<>>, (indList))

(*
  @type: ((Set(a48), ((a48, a48) => EQ({ tag: Str }) | GT({ tag: Str }) | LT({ tag: Str }))) => Seq(a48));
*)
fancy_toList(fancy___set_2557, fancy___cmp_2557(_, _)) ==
  LET (*
    @type: ((Seq(a48), a48) => Seq(a48));
  *)
  __QUINT_LAMBDA170(fancy___l_2555, fancy___e_2555) ==
    fancy_sortedListInsert(fancy___l_2555, fancy___e_2555, fancy___cmp_2557)
  IN
  ApaFoldSet(__QUINT_LAMBDA170, <<>>, fancy___set_2557)

(*
  @type: ((Int, Int) => EQ({ tag: Str }) | GT({ tag: Str }) | LT({ tag: Str }));
*)
fancy_intCompare(fancy___a_2279, fancy___b_2279) ==
  IF fancy___a_2279 < fancy___b_2279
  THEN fancy_LT
  ELSE IF fancy___a_2279 > fancy___b_2279 THEN fancy_GT ELSE fancy_EQ

(*
  @type: (((a49 -> a50), a49) => None({ tag: Str }) | Some(a49));
*)
fancy_safeGet(fancy_m_4096, fancy_k_4096) ==
  IF fancy_has(fancy_m_4096, fancy_k_4096)
  THEN fancy_Some(fancy_m_4096[fancy_k_4096])
  ELSE fancy_None

(*
  @type: ((Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) })) => <<Set({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })>>);
*)
fancy_prepare_batch_for_subtree(fancy_batch_6840, fancy_existing_leaf_6840) ==
  LET (*
    @type: (() => None({ tag: Str }) | Some({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }));
  *)
  fancy_maybe_op ==
    LET (*
      @type: ((None({ tag: Str }) | Some({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }), { key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }) => None({ tag: Str }) | Some({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }));
    *)
    __QUINT_LAMBDA177(fancy_acc_6802, fancy_op_6802) ==
      CASE VariantTag(fancy_existing_leaf_6840) = "Some"
          -> LET (*
            @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => None({ tag: Str }) | Some({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }));
          *)
          __QUINT_LAMBDA175(fancy_leaf_6797) ==
            IF fancy_op_6802["key_hash"] = fancy_leaf_6797["key_hash"]
            THEN fancy_Some(fancy_op_6802)
            ELSE fancy_acc_6802
          IN
          __QUINT_LAMBDA175(VariantGetUnsafe("Some", fancy_existing_leaf_6840))
        [] VariantTag(fancy_existing_leaf_6840) = "None"
          -> LET (*
            @type: (({ tag: Str }) => None({ tag: Str }) | Some({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }));
          *)
          __QUINT_LAMBDA176(fancy___6800) == fancy_acc_6802
          IN
          __QUINT_LAMBDA176(VariantGetUnsafe("None", fancy_existing_leaf_6840))
    IN
    ApaFoldSet(__QUINT_LAMBDA177, (fancy_None), fancy_batch_6840)
  IN
  LET (*
    @type: (() => Set({ key_hash: Seq(Int), value_hash: Seq(Int) }));
  *)
  fancy_filtered_batch ==
    LET (*
      @type: (({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }) => None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) }));
    *)
    __QUINT_LAMBDA180(fancy_op_6832) ==
      IF fancy_maybe_op = fancy_Some(fancy_op_6832)
      THEN fancy_None
      ELSE CASE VariantTag(fancy_op_6832["op"]) = "Insert"
          -> LET (*
            @type: ((Seq(Int)) => None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) }));
          *)
          __QUINT_LAMBDA178(fancy_value_hash_6826) ==
            fancy_Some([key_hash |-> fancy_op_6832["key_hash"],
              value_hash |-> fancy_value_hash_6826])
          IN
          __QUINT_LAMBDA178(VariantGetUnsafe("Insert", fancy_op_6832["op"]))
        [] OTHER
          -> LET (*
            @type: ((a55) => None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) }));
          *)
          __QUINT_LAMBDA179(fancy___6829) == fancy_None
          IN
          __QUINT_LAMBDA179({})
    IN
    fancy_filterMap(fancy_batch_6840, __QUINT_LAMBDA180)
  IN
  <<(fancy_filtered_batch), (fancy_maybe_op)>>

(*
  @type: ((None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) }), Seq(Int)) => <<None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) })>>);
*)
fancy_partition_leaf(fancy_leaf_6763, fancy_bits_6763) ==
  CASE VariantTag(fancy_leaf_6763) = "Some"
      -> LET (*
        @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => <<None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) })>>);
      *)
      __QUINT_LAMBDA184(fancy_leaf_6758) ==
        IF fancy_leaf_6758["key_hash"][(Len(fancy_bits_6763) + 1)] = 0
        THEN <<(fancy_Some(fancy_leaf_6758)), (fancy_None)>>
        ELSE <<(fancy_None), (fancy_Some(fancy_leaf_6758))>>
      IN
      __QUINT_LAMBDA184(VariantGetUnsafe("Some", fancy_leaf_6763))
    [] VariantTag(fancy_leaf_6763) = "None"
      -> LET (*
        @type: (({ tag: Str }) => <<None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) })>>);
      *)
      __QUINT_LAMBDA185(fancy___6761) == <<(fancy_None), (fancy_None)>>
      IN
      __QUINT_LAMBDA185(VariantGetUnsafe("None", fancy_leaf_6763))

(*
  @type: ((Seq(a65), a66, ((a65, a66) => a66)) => a66);
*)
foldr(l_7816, init_7816, op_7816(_, _)) ==
  LET (*
    @type: ((a66, a65) => a66);
  *)
  __QUINT_LAMBDA202(acc_7814, e_7814) == op_7816(e_7814, acc_7814)
  IN
  ApaFoldSeqLeft(__QUINT_LAMBDA202, init_7816, (reverse(l_7816)))

(*
  @type: ((Seq(a69), a70, ((a69, a70) => a70)) => a70);
*)
fancy_foldr(fancy_l_7816, fancy_init_7816, fancy_op_7816(_, _)) ==
  LET (*
    @type: ((a70, a69) => a70);
  *)
  __QUINT_LAMBDA256(fancy_acc_7814, fancy_e_7814) ==
    fancy_op_7816(fancy_e_7814, fancy_acc_7814)
  IN
  ApaFoldSeqLeft(__QUINT_LAMBDA256, fancy_init_7816, (reverse(fancy_l_7816)))

(*
  @type: (() => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
Hash256_ZERO ==
  raw(<<
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0
  >>)

(*
  @type: (((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), Int, Int) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
termSlice(term_3205, start_3205, end_3205) ==
  IF Cardinality((DOMAIN term_3205)) /= 1
  THEN term_3205
  ELSE LET (*
    @type: (() => Hash({ tag: Str }) | Raw(Seq(Int)));
  *)
  first == term_3205[<<0>>]
  IN
  CASE VariantTag((first)) = "Raw"
      -> LET (*
        @type: ((Seq(Int)) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
      *)
      __QUINT_LAMBDA9(bytes_3198) ==
        raw((SubSeq(bytes_3198, (start_3205 + 1), end_3205)))
      IN
      __QUINT_LAMBDA9(VariantGetUnsafe("Raw", (first)))
    [] OTHER
      -> LET (*
        @type: ((l) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
      *)
      __QUINT_LAMBDA10(id__3201) == term_3205
      IN
      __QUINT_LAMBDA10({})

(*
  @type: (((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
termConcat(left_3169, right_3169) ==
  LET (*
    @type: (() => Hash({ tag: Str }) | Raw(Seq(Int)));
  *)
  l == IF isRaw(left_3169) THEN left_3169[<<0>>] ELSE Hash
  IN
  LET (*
    @type: (() => Hash({ tag: Str }) | Raw(Seq(Int)));
  *)
  r == IF isRaw(right_3169) THEN right_3169[<<0>>] ELSE Hash
  IN
  LET (*
    @type: (((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
  *)
  mergeTerms(left_3117, right_3117) ==
    LET (*
      @type: (() => Int);
    *)
    lwidth == Cardinality({ p_3060 \in DOMAIN left_3117: Len(p_3060) = 1 })
    IN
    LET (*
      @type: (() => Set(Seq(Int)));
    *)
    rshifted ==
      {
        <<(lwidth + p_3079[(0 + 1)])>> \o SubSeq(p_3079, (1 + 1), (Len(p_3079))):
          p_3079 \in DOMAIN right_3117
      }
    IN
    LET (*
      @type: (() => Set(Seq(Int)));
    *)
    paths == DOMAIN left_3117 \union rshifted
    IN
    [
      p_3112 \in paths |->
        IF p_3112[(0 + 1)] < lwidth
        THEN left_3117[p_3112]
        ELSE right_3117[
          (<<(p_3112[(0 + 1)] - lwidth)>>
            \o SubSeq(p_3112, (1 + 1), (Len(p_3112))))
        ]
    ]
  IN
  CASE VariantTag((l)) = "Raw"
      -> LET (*
        @type: ((Seq(Int)) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
      *)
      __QUINT_LAMBDA13(lBytes_3161) ==
        CASE VariantTag((r)) = "Raw"
            -> LET (*
              @type: ((Seq(Int)) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
            *)
            __QUINT_LAMBDA11(rBytes_3135) == raw((lBytes_3161 \o rBytes_3135))
            IN
            __QUINT_LAMBDA11(VariantGetUnsafe("Raw", (r)))
          [] VariantTag((r)) = "Hash"
            -> LET (*
              @type: (({ tag: Str }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
            *)
            __QUINT_LAMBDA12(id__3138) ==
              IF lBytes_3161 = <<>>
              THEN right_3169
              ELSE mergeTerms(left_3169, right_3169)
            IN
            __QUINT_LAMBDA12(VariantGetUnsafe("Hash", (r)))
      IN
      __QUINT_LAMBDA13(VariantGetUnsafe("Raw", (l)))
    [] VariantTag((l)) = "Hash"
      -> LET (*
        @type: (({ tag: Str }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
      *)
      __QUINT_LAMBDA16(id__3164) ==
        CASE VariantTag((r)) = "Raw"
            -> LET (*
              @type: ((Seq(Int)) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
            *)
            __QUINT_LAMBDA14(rBytes_3154) ==
              IF rBytes_3154 = <<>>
              THEN left_3169
              ELSE mergeTerms(left_3169, right_3169)
            IN
            __QUINT_LAMBDA14(VariantGetUnsafe("Raw", (r)))
          [] VariantTag((r)) = "Hash"
            -> LET (*
              @type: (({ tag: Str }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
            *)
            __QUINT_LAMBDA15(id__3157) == mergeTerms(left_3169, right_3169)
            IN
            __QUINT_LAMBDA15(VariantGetUnsafe("Hash", (r)))
      IN
      __QUINT_LAMBDA16(VariantGetUnsafe("Hash", (l)))

(*
  @type: (() => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
InternalNodeHashPrefix == raw((InternalNodeIdentifier))

(*
  @type: (() => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
LeafNodeHashPrefix == raw((LeafNodeIdentifier))

(*
  @type: (() => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
fancy_Hash256_ZERO ==
  fancy_raw(<<
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0
  >>)

(*
  @type: (((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
fancy_termConcat(fancy_left_3169, fancy_right_3169) ==
  LET (*
    @type: (() => Hash({ tag: Str }) | Raw(Seq(Int)));
  *)
  fancy_l ==
    IF fancy_isRaw(fancy_left_3169) THEN fancy_left_3169[<<0>>] ELSE fancy_Hash
  IN
  LET (*
    @type: (() => Hash({ tag: Str }) | Raw(Seq(Int)));
  *)
  fancy_r ==
    IF fancy_isRaw(fancy_right_3169)
    THEN fancy_right_3169[<<0>>]
    ELSE fancy_Hash
  IN
  LET (*
    @type: (((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
  *)
  fancy_mergeTerms(fancy_left_3117, fancy_right_3117) ==
    LET (*
      @type: (() => Int);
    *)
    fancy_lwidth ==
      Cardinality({
        fancy_p_3060 \in DOMAIN fancy_left_3117:
          Len(fancy_p_3060) = 1
      })
    IN
    LET (*
      @type: (() => Set(Seq(Int)));
    *)
    fancy_rshifted ==
      {
        <<(fancy_lwidth + fancy_p_3079[(0 + 1)])>>
          \o SubSeq(fancy_p_3079, (1 + 1), (Len(fancy_p_3079))):
          fancy_p_3079 \in DOMAIN fancy_right_3117
      }
    IN
    LET (*
      @type: (() => Set(Seq(Int)));
    *)
    fancy_paths == DOMAIN fancy_left_3117 \union fancy_rshifted
    IN
    [
      fancy_p_3112 \in fancy_paths |->
        IF fancy_p_3112[(0 + 1)] < fancy_lwidth
        THEN fancy_left_3117[fancy_p_3112]
        ELSE fancy_right_3117[
          (<<(fancy_p_3112[(0 + 1)] - fancy_lwidth)>>
            \o SubSeq(fancy_p_3112, (1 + 1), (Len(fancy_p_3112))))
        ]
    ]
  IN
  CASE VariantTag((fancy_l)) = "Raw"
      -> LET (*
        @type: ((Seq(Int)) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
      *)
      __QUINT_LAMBDA21(fancy_lBytes_3161) ==
        CASE VariantTag((fancy_r)) = "Raw"
            -> LET (*
              @type: ((Seq(Int)) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
            *)
            __QUINT_LAMBDA19(fancy_rBytes_3135) ==
              fancy_raw((fancy_lBytes_3161 \o fancy_rBytes_3135))
            IN
            __QUINT_LAMBDA19(VariantGetUnsafe("Raw", (fancy_r)))
          [] VariantTag((fancy_r)) = "Hash"
            -> LET (*
              @type: (({ tag: Str }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
            *)
            __QUINT_LAMBDA20(fancy___3138) ==
              IF fancy_lBytes_3161 = <<>>
              THEN fancy_right_3169
              ELSE fancy_mergeTerms(fancy_left_3169, fancy_right_3169)
            IN
            __QUINT_LAMBDA20(VariantGetUnsafe("Hash", (fancy_r)))
      IN
      __QUINT_LAMBDA21(VariantGetUnsafe("Raw", (fancy_l)))
    [] VariantTag((fancy_l)) = "Hash"
      -> LET (*
        @type: (({ tag: Str }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
      *)
      __QUINT_LAMBDA24(fancy___3164) ==
        CASE VariantTag((fancy_r)) = "Raw"
            -> LET (*
              @type: ((Seq(Int)) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
            *)
            __QUINT_LAMBDA22(fancy_rBytes_3154) ==
              IF fancy_rBytes_3154 = <<>>
              THEN fancy_left_3169
              ELSE fancy_mergeTerms(fancy_left_3169, fancy_right_3169)
            IN
            __QUINT_LAMBDA22(VariantGetUnsafe("Raw", (fancy_r)))
          [] VariantTag((fancy_r)) = "Hash"
            -> LET (*
              @type: (({ tag: Str }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
            *)
            __QUINT_LAMBDA23(fancy___3157) ==
              fancy_mergeTerms(fancy_left_3169, fancy_right_3169)
            IN
            __QUINT_LAMBDA23(VariantGetUnsafe("Hash", (fancy_r)))
      IN
      __QUINT_LAMBDA24(VariantGetUnsafe("Hash", (fancy_l)))

(*
  @type: (() => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
fancy_LeafNodeHashPrefix == fancy_raw((fancy_LeafNodeIdentifier))

(*
  @type: (() => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
fancy_InternalNodeHashPrefix == fancy_raw((fancy_InternalNodeIdentifier))

(*
  @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
*)
addDirectChildren(t_3678, pool_3678) ==
  LET (*
    @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), Seq(Int)) => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
  *)
  __QUINT_LAMBDA57(treeNodes_3676, prefix_3676) ==
    CASE VariantTag((mostRecentNodeId(pool_3678, prefix_3676))) = "Some"
        -> LET (*
          @type: (({ key_hash: Seq(Int), version: Int }) => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
        *)
        __QUINT_LAMBDA55(nodeId_3671) ==
          LET (*
            @type: (() => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
          *)
          __quint_var13 == treeNodes_3676
          IN
          LET (*
            @type: (() => Set({ key_hash: Seq(Int), version: Int }));
          *)
          __quint_var14 == DOMAIN __quint_var13
          IN
          [
            __quint_var15 \in {nodeId_3671} \union __quint_var14 |->
              IF __quint_var15 = nodeId_3671
              THEN pool_3678[nodeId_3671]
              ELSE (__quint_var13)[__quint_var15]
          ]
        IN
        __QUINT_LAMBDA55(VariantGetUnsafe("Some", (mostRecentNodeId(pool_3678, prefix_3676))))
      [] VariantTag((mostRecentNodeId(pool_3678, prefix_3676))) = "None"
        -> LET (*
          @type: (({ tag: Str }) => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
        *)
        __QUINT_LAMBDA56(id__3674) == treeNodes_3676
        IN
        __QUINT_LAMBDA56(VariantGetUnsafe("None", (mostRecentNodeId(pool_3678, prefix_3676))))
  IN
  ApaFoldSet(__QUINT_LAMBDA57, t_3678, (childrenPrefixes(t_3678)))

(*
  @type: (({ child_order: Seq(Int), child_size: Int, empty_child: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), max_prefix_length: Int, min_prefix_length: Int }, Int) => None({ tag: Str }) | Some({ max_prefix: Int, min_prefix: Int, suffix: Int }));
*)
get_padding(spec_4715, branch_4715) ==
  CASE VariantTag((LET (*
      @type: ((Int) => Bool);
    *)
    __QUINT_LAMBDA112(x_4668) == x_4668 = branch_4715
    IN
    findFirst(spec_4715["child_order"], __QUINT_LAMBDA112)))
      = "Some"
      -> LET (*
        @type: ((Int) => None({ tag: Str }) | Some({ max_prefix: Int, min_prefix: Int, suffix: Int }));
      *)
      __QUINT_LAMBDA113(idx_4710) ==
        LET (*
          @type: (() => Int);
        *)
        prefix == idx_4710 * spec_4715["child_size"]
        IN
        LET (*
          @type: (() => Int);
        *)
        suffix ==
          spec_4715["child_size"]
            * ((Len(spec_4715["child_order"]) - 1) - idx_4710)
        IN
        Some([min_prefix |-> prefix + spec_4715["min_prefix_length"],
          max_prefix |-> prefix + spec_4715["max_prefix_length"],
          suffix |-> suffix])
      IN
      __QUINT_LAMBDA113(VariantGetUnsafe("Some", (LET (*
        @type: ((Int) => Bool);
      *)
      __QUINT_LAMBDA112(x_4668) == x_4668 = branch_4715
      IN
      findFirst(spec_4715["child_order"], __QUINT_LAMBDA112))))
    [] VariantTag((LET (*
      @type: ((Int) => Bool);
    *)
    __QUINT_LAMBDA112(x_4668) == x_4668 = branch_4715
    IN
    findFirst(spec_4715["child_order"], __QUINT_LAMBDA112)))
      = "None"
      -> LET (*
        @type: (({ tag: Str }) => None({ tag: Str }) | Some({ max_prefix: Int, min_prefix: Int, suffix: Int }));
      *)
      __QUINT_LAMBDA114(id__4713) == None
      IN
      __QUINT_LAMBDA114(VariantGetUnsafe("None", (LET (*
        @type: ((Int) => Bool);
      *)
      __QUINT_LAMBDA112(x_4668) == x_4668 = branch_4715
      IN
      findFirst(spec_4715["child_order"], __QUINT_LAMBDA112))))

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }) => Seq({ key_hash: Seq(Int), version: Int }));
*)
fancy_sorted_nodes(fancy_tree_5783) ==
  LET (*
    @type: (({ key_hash: Seq(Int), version: Int }, { key_hash: Seq(Int), version: Int }) => EQ({ tag: Str }) | GT({ tag: Str }) | LT({ tag: Str }));
  *)
  __QUINT_LAMBDA171(fancy_a_5781, fancy_b_5781) ==
    fancy_intCompare((Len(fancy_a_5781["key_hash"])), (Len(fancy_b_5781[
      "key_hash"
    ])))
  IN
  fancy_toList(DOMAIN (fancy_tree_5783["nodes"]), __QUINT_LAMBDA171)

(*
  @type: (((({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, Int, Int, Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })) => { nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }), ((Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }), { nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }) => Bool)) => Bool);
*)
step_parametrized(apply_op_157(_, _, _, _), assign_result_157(_, _)) ==
  version <= 2
    /\ (\E kms_with_value \in [(all_key_hashes) -> (VALUES)]:
      LET (*
        @type: (() => Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }));
      *)
      ops == to_operations(kms_with_value)
      IN
      LET (*
        @type: (() => { nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) });
      *)
      new_tree == apply_op_157(tree, (version - 1), version, (ops))
      IN
      assign_result_157((ops), (new_tree)))

(*
  @type: ((None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int })) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
hashOfChild(oc_3985) ==
  CASE VariantTag(oc_3985) = "None"
      -> LET (*
        @type: (({ tag: Str }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
      *)
      __QUINT_LAMBDA17(id__3980) == Hash256_ZERO
      IN
      __QUINT_LAMBDA17(VariantGetUnsafe("None", oc_3985))
    [] VariantTag(oc_3985) = "Some"
      -> LET (*
        @type: (({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
      *)
      __QUINT_LAMBDA18(c_3983) == c_3983["hash"]
      IN
      __QUINT_LAMBDA18(VariantGetUnsafe("Some", oc_3985))

(*
  @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
hashLeafNode(l_3966) ==
  termHash((termConcat((termConcat((LeafNodeHashPrefix), (raw(l_3966["key_hash"])))),
  (raw(l_3966["value_hash"])))))

(*
  @type: ((None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int })) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
fancy_hashOfChild(fancy_oc_3985) ==
  CASE VariantTag(fancy_oc_3985) = "None"
      -> LET (*
        @type: (({ tag: Str }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
      *)
      __QUINT_LAMBDA25(fancy___3980) == fancy_Hash256_ZERO
      IN
      __QUINT_LAMBDA25(VariantGetUnsafe("None", fancy_oc_3985))
    [] VariantTag(fancy_oc_3985) = "Some"
      -> LET (*
        @type: (({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
      *)
      __QUINT_LAMBDA26(fancy_c_3983) == fancy_c_3983["hash"]
      IN
      __QUINT_LAMBDA26(VariantGetUnsafe("Some", fancy_oc_3985))

(*
  @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
fancy_hashLeafNode(fancy_l_3966) ==
  fancy_termHash((fancy_termConcat((fancy_termConcat((fancy_LeafNodeHashPrefix),
  (fancy_raw(fancy_l_3966["key_hash"])))), (fancy_raw(fancy_l_3966["value_hash"])))))

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, Int) => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
*)
treeAtVersion(t_3710, version_3710) ==
  LET (*
    @type: (() => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
  *)
  startingNodes == nodesAtVersion(t_3710["nodes"], version_3710)
  IN
  LET (*
    @type: (() => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
  *)
  nodePool == nodesUpToVersion(t_3710["nodes"], version_3710)
  IN
  LET (*
    @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), Int) => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
  *)
  __QUINT_LAMBDA58(treeNodes_3706, id__3706) ==
    addDirectChildren(treeNodes_3706, (nodePool))
  IN
  ApaFoldSet(__QUINT_LAMBDA58, (startingNodes), (0 .. MAX_HASH_LENGTH))

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, Int, Seq(Int)) => None({ tag: Str }) | Some(Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) })));
*)
ics23_prove_existence(t_5367, version_5367, key_hash_5367) ==
  LET (*
    @type: (() => Seq(Seq(Int)));
  *)
  prefixes_list ==
    toList({
      SubSeq(key_hash_5367, (0 + 1), i_5183):
        i_5183 \in 0 .. Len(key_hash_5367)
    }, listCompare)
  IN
  LET (*
    @type: (() => { child_version: Int, found: Bool, i: Int, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }) });
  *)
  r ==
    LET (*
      @type: (({ child_version: Int, found: Bool, i: Int, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }) }, Seq(Int)) => { child_version: Int, found: Bool, i: Int, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }) });
    *)
    __QUINT_LAMBDA75(iterator_5352, key_prefix_5352) ==
      IF iterator_5352["found"]
        \/ ~([key_hash |-> key_prefix_5352,
          version |-> iterator_5352["child_version"]]
          \in DOMAIN (t_5367["nodes"]))
      THEN iterator_5352
      ELSE LET (*
        @type: (() => Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }));
      *)
      node ==
        t_5367["nodes"][
          [key_hash |-> key_prefix_5352,
            version |-> iterator_5352["child_version"]]
        ]
      IN
      CASE VariantTag((node)) = "Leaf"
          -> LET (*
            @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => { child_version: Int, found: Bool, i: Int, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }) });
          *)
          __QUINT_LAMBDA69(l_5345) ==
            [
              [ iterator_5352 EXCEPT !["i"] = iterator_5352["i"] + 1 ] EXCEPT
                !["found"] = l_5345["key_hash"] = key_hash_5367
            ]
          IN
          __QUINT_LAMBDA69(VariantGetUnsafe("Leaf", (node)))
        [] VariantTag((node)) = "Internal"
          -> LET (*
            @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => { child_version: Int, found: Bool, i: Int, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }) });
          *)
          __QUINT_LAMBDA74(internal_5348) ==
            LET (*
              @type: (() => Seq(Int));
            *)
            next_bit_0 == Append(key_prefix_5352, 0)
            IN
            LET (*
              @type: (() => Int);
            *)
            child_version ==
              IF (prefixes_list)[((iterator_5352["i"] + 1) + 1)] = next_bit_0
              THEN (unwrap(internal_5348["left_child"]))["version"]
              ELSE (unwrap(internal_5348["right_child"]))["version"]
            IN
            LET (*
              @type: (() => { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) });
            *)
            innerOp ==
              IF (prefixes_list)[((iterator_5352["i"] + 1) + 1)] = next_bit_0
              THEN [prefix |-> InternalNodeHashPrefix,
                suffix |->
                  CASE VariantTag(internal_5348["right_child"]) = "None"
                      -> LET (*
                        @type: (({ tag: Str }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
                      *)
                      __QUINT_LAMBDA70(id__5293) == Hash256_ZERO
                      IN
                      __QUINT_LAMBDA70(VariantGetUnsafe("None", internal_5348[
                        "right_child"
                      ]))
                    [] VariantTag(internal_5348["right_child"]) = "Some"
                      -> LET (*
                        @type: (({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
                      *)
                      __QUINT_LAMBDA71(c_5296) == c_5296["hash"]
                      IN
                      __QUINT_LAMBDA71(VariantGetUnsafe("Some", internal_5348[
                        "right_child"
                      ]))]
              ELSE [prefix |->
                  termConcat((InternalNodeHashPrefix), (CASE VariantTag(internal_5348[
                      "left_child"
                    ])
                      = "None"
                      -> LET (*
                        @type: (({ tag: Str }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
                      *)
                      __QUINT_LAMBDA72(id__5310) == Hash256_ZERO
                      IN
                      __QUINT_LAMBDA72(VariantGetUnsafe("None", internal_5348[
                        "left_child"
                      ]))
                    [] VariantTag(internal_5348["left_child"]) = "Some"
                      -> LET (*
                        @type: (({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
                      *)
                      __QUINT_LAMBDA73(c_5313) == c_5313["hash"]
                      IN
                      __QUINT_LAMBDA73(VariantGetUnsafe("Some", internal_5348[
                        "left_child"
                      ])))),
                suffix |-> SetAsFun({})]
            IN
            [
              [
                [
                  iterator_5352 EXCEPT
                    !["path"] = Append(iterator_5352["path"], (innerOp))
                ] EXCEPT
                  !["i"] = iterator_5352["i"] + 1
              ] EXCEPT
                !["child_version"] = child_version
            ]
          IN
          __QUINT_LAMBDA74(VariantGetUnsafe("Internal", (node)))
    IN
    ApaFoldSeqLeft(__QUINT_LAMBDA75, [path |-> <<>>,
      i |-> 0,
      found |-> FALSE,
      child_version |-> version_5367], (prefixes_list))
  IN
  IF (r)["found"] THEN Some((reverse((r)["path"]))) ELSE None

(*
  @type: (({ child_order: Seq(Int), child_size: Int, empty_child: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), max_prefix_length: Int, min_prefix_length: Int }, { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }) => None({ tag: Str }) | Some(Int));
*)
order_from_padding(spec_4862, op_4862) ==
  LET (*@type: (() => Int); *) len == Len(spec_4862["child_order"]) IN
  LET (*
    @type: ((Int) => Bool);
  *)
  __QUINT_LAMBDA119(branch_4859) ==
    CASE VariantTag((get_padding(spec_4862, branch_4859))) = "Some"
        -> LET (*
          @type: (({ max_prefix: Int, min_prefix: Int, suffix: Int }) => Bool);
        *)
        __QUINT_LAMBDA117(padding_4854) == has_padding(op_4862, padding_4854)
        IN
        __QUINT_LAMBDA117(VariantGetUnsafe("Some", (get_padding(spec_4862, branch_4859))))
      [] VariantTag((get_padding(spec_4862, branch_4859))) = "None"
        -> LET (*
          @type: (({ tag: Str }) => Bool);
        *)
        __QUINT_LAMBDA118(id__4857) == FALSE
        IN
        __QUINT_LAMBDA118(VariantGetUnsafe("None", (get_padding(spec_4862, branch_4859))))
  IN
  find_first(LET (*
    @type: ((Int) => Int);
  *)
  __QUINT_LAMBDA116(__quint_var17) == (0 + __quint_var17) - 1
  IN
  MkSeq((len - 0), __QUINT_LAMBDA116), __QUINT_LAMBDA119)

(*
  @type: (() => { child_order: Seq(Int), child_size: Int, empty_child: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), max_prefix_length: Int, min_prefix_length: Int });
*)
ics23_InnerSpec ==
  [min_prefix_length |-> 1,
    max_prefix_length |-> 1,
    child_size |-> 32,
    empty_child |-> Hash256_ZERO,
    child_order |-> <<0, 1>>]

(*
  @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
hashInternalNode(l_4002) ==
  termHash((termConcat((termConcat((InternalNodeHashPrefix), (hashOfChild(l_4002[
    "left_child"
  ])))), (hashOfChild(l_4002["right_child"])))))

(*
  @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
fancy_hashInternalNode(fancy_l_4002) ==
  fancy_termHash((fancy_termConcat((fancy_termConcat((fancy_InternalNodeHashPrefix),
  (fancy_hashOfChild(fancy_l_4002["left_child"])))), (fancy_hashOfChild(fancy_l_4002[
    "right_child"
  ])))))

(*
  @type: (() => Set(({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))));
*)
treesToCheck == { treeAtVersion(tree, v_207): v_207 \in versionsToCheck }

(*
  @type: (() => Bool);
*)
orphansInNoTreeInv ==
  \A o_664 \in tree["orphans"]:
    LET (*
      @type: (() => { key_hash: Seq(Int), version: Int });
    *)
    nodeId == [version |-> o_664["version"], key_hash |-> o_664["key_hash"]]
    IN
    \A ver_661 \in o_664["orphaned_since_version"] .. treeVersion(tree):
      ~(nodeId \in DOMAIN (treeAtVersion(tree, ver_661)))

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, Seq(Int), Int) => None({ tag: Str }) | Some(Exist({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) | NonExist({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) })));
*)
ics23_prove(t_5734, key_hash_5734, version_5734) ==
  LET (*
    @type: (() => Set(None({ tag: Str }) | Some(Seq(Int))));
  *)
  optionalValueForKey ==
    {
      Some(l_5559["value_hash"]):
        l_5559 \in
          {
            l_5552 \in allLeafs((treeAtVersion(t_5734, version_5734))):
              l_5552["key_hash"] = key_hash_5734
          }
    }
  IN
  LET (*
    @type: (() => None({ tag: Str }) | Some(Seq(Int)));
  *)
  state_storage_read ==
    IF empty((optionalValueForKey))
    THEN None
    ELSE getOnlyElement((optionalValueForKey))
  IN
  CASE VariantTag((state_storage_read)) = "Some"
      -> LET (*
        @type: ((Seq(Int)) => None({ tag: Str }) | Some(Exist({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) | NonExist({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) })));
      *)
      __QUINT_LAMBDA94(value_5727) ==
        LET (*
          @type: (() => None({ tag: Str }) | Some(Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) })));
        *)
        p == ics23_prove_existence(t_5734, version_5734, key_hash_5734)
        IN
        CASE VariantTag((p)) = "Some"
            -> LET (*
              @type: ((Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) })) => None({ tag: Str }) | Some(Exist({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) | NonExist({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) })));
            *)
            __QUINT_LAMBDA92(path_5592) ==
              Some((Exist([key |-> key_hash_5734,
                value |-> value_5727,
                leaf |-> [prefix |-> LeafNodeHashPrefix],
                path |-> path_5592])))
            IN
            __QUINT_LAMBDA92(VariantGetUnsafe("Some", (p)))
          [] VariantTag((p)) = "None"
            -> LET (*
              @type: (({ tag: Str }) => None({ tag: Str }) | Some(Exist({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) | NonExist({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) })));
            *)
            __QUINT_LAMBDA93(id__5595) == None
            IN
            __QUINT_LAMBDA93(VariantGetUnsafe("None", (p)))
      IN
      __QUINT_LAMBDA94(VariantGetUnsafe("Some", (state_storage_read)))
    [] VariantTag((state_storage_read)) = "None"
      -> LET (*
        @type: (({ tag: Str }) => None({ tag: Str }) | Some(Exist({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) | NonExist({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) })));
      *)
      __QUINT_LAMBDA103(id__5730) ==
        LET (*
          @type: (() => None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) }));
        *)
        lneighborOption ==
          leftNeighbor((treeAtVersion(t_5734, version_5734)), key_hash_5734)
        IN
        LET (*
          @type: (() => None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }));
        *)
        leftNeighborExistenceProof ==
          CASE VariantTag((lneighborOption)) = "Some"
              -> LET (*
                @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }));
              *)
              __QUINT_LAMBDA97(lneighbor_5647) ==
                LET (*
                  @type: (() => None({ tag: Str }) | Some(Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) })));
                *)
                pathOption ==
                  ics23_prove_existence(t_5734, version_5734, lneighbor_5647[
                    "key_hash"
                  ])
                IN
                CASE VariantTag((pathOption)) = "Some"
                    -> LET (*
                      @type: ((Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) })) => None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }));
                    *)
                    __QUINT_LAMBDA95(path_5638) ==
                      Some([key |-> lneighbor_5647["key_hash"],
                        value |-> lneighbor_5647["value_hash"],
                        leaf |-> [prefix |-> LeafNodeHashPrefix],
                        path |-> path_5638])
                    IN
                    __QUINT_LAMBDA95(VariantGetUnsafe("Some", (pathOption)))
                  [] VariantTag((pathOption)) = "None"
                    -> LET (*
                      @type: (({ tag: Str }) => None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }));
                    *)
                    __QUINT_LAMBDA96(id__5641) == None
                    IN
                    __QUINT_LAMBDA96(VariantGetUnsafe("None", (pathOption)))
              IN
              __QUINT_LAMBDA97(VariantGetUnsafe("Some", (lneighborOption)))
            [] VariantTag((lneighborOption)) = "None"
              -> LET (*
                @type: (({ tag: Str }) => None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }));
              *)
              __QUINT_LAMBDA98(id__5650) == None
              IN
              __QUINT_LAMBDA98(VariantGetUnsafe("None", (lneighborOption)))
        IN
        LET (*
          @type: (() => None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) }));
        *)
        rneighborOption ==
          rightNeighbor((treeAtVersion(t_5734, version_5734)), key_hash_5734)
        IN
        LET (*
          @type: (() => None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }));
        *)
        rightNeighborExistenceProof ==
          CASE VariantTag((rneighborOption)) = "Some"
              -> LET (*
                @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }));
              *)
              __QUINT_LAMBDA101(rneighbor_5702) ==
                LET (*
                  @type: (() => None({ tag: Str }) | Some(Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) })));
                *)
                pathOption ==
                  ics23_prove_existence(t_5734, version_5734, rneighbor_5702[
                    "key_hash"
                  ])
                IN
                CASE VariantTag((pathOption)) = "Some"
                    -> LET (*
                      @type: ((Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) })) => None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }));
                    *)
                    __QUINT_LAMBDA99(path_5693) ==
                      Some([key |-> rneighbor_5702["key_hash"],
                        value |-> rneighbor_5702["value_hash"],
                        leaf |-> [prefix |-> LeafNodeHashPrefix],
                        path |-> path_5693])
                    IN
                    __QUINT_LAMBDA99(VariantGetUnsafe("Some", (pathOption)))
                  [] VariantTag((pathOption)) = "None"
                    -> LET (*
                      @type: (({ tag: Str }) => None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }));
                    *)
                    __QUINT_LAMBDA100(id__5696) == None
                    IN
                    __QUINT_LAMBDA100(VariantGetUnsafe("None", (pathOption)))
              IN
              __QUINT_LAMBDA101(VariantGetUnsafe("Some", (rneighborOption)))
            [] VariantTag((rneighborOption)) = "None"
              -> LET (*
                @type: (({ tag: Str }) => None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }));
              *)
              __QUINT_LAMBDA102(id__5705) == None
              IN
              __QUINT_LAMBDA102(VariantGetUnsafe("None", (rneighborOption)))
        IN
        LET (*
          @type: (() => { key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) });
        *)
        nep ==
          [key |-> key_hash_5734,
            left |-> leftNeighborExistenceProof,
            right |-> rightNeighborExistenceProof]
        IN
        Some((NonExist((nep))))
      IN
      __QUINT_LAMBDA103(VariantGetUnsafe("None", (state_storage_read)))

(*
  @type: (({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
exists_calculate(p_4519) ==
  LET (*
    @type: (() => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
  *)
  leafHash ==
    hashLeafNode([key_hash |-> p_4519["key"], value_hash |-> p_4519["value"]])
  IN
  LET (*
    @type: (((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
  *)
  __QUINT_LAMBDA104(child_4516, inner_4516) ==
    termHash((termConcat((termConcat(inner_4516["prefix"], child_4516)), inner_4516[
      "suffix"
    ])))
  IN
  ApaFoldSeqLeft(__QUINT_LAMBDA104, (leafHash), p_4519["path"])

(*
  @type: (({ child_order: Seq(Int), child_size: Int, empty_child: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), max_prefix_length: Int, min_prefix_length: Int }, { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }) => Bool);
*)
left_branches_are_empty(spec_4944, op_4944) ==
  LET (*
      @type: (() => None({ tag: Str }) | Some(Int));
    *)
    idx == order_from_padding(spec_4944, op_4944)
    IN
    LET (*@type: (() => Int); *) left_branches == unwrap((idx)) IN
    IF left_branches = 0
    THEN FALSE
    ELSE LET (*@type: (() => Int); *) child_size == spec_4944["child_size"] IN
    CASE VariantTag((checked_sub((termLen(op_4944["prefix"])), (left_branches
        * child_size))))
        = "Some"
        -> LET (*
          @type: ((Int) => Bool);
        *)
        __QUINT_LAMBDA121(actual_prefix_4934) ==
          \A i_4929 \in 0 .. left_branches - 1:
            LET (*
              @type: (() => Int);
            *)
            idx_4928 ==
              unwrap((LET (*
                @type: ((Int) => Bool);
              *)
              __QUINT_LAMBDA120(x_4905) == x_4905 = i_4929
              IN
              findFirst(spec_4944["child_order"], __QUINT_LAMBDA120)))
            IN
            LET (*
              @type: (() => Int);
            *)
            from_index == actual_prefix_4934 + idx_4928 * child_size
            IN
            spec_4944["empty_child"]
              = termSlice(op_4944["prefix"], (from_index), (from_index
                + child_size))
        IN
        __QUINT_LAMBDA121(VariantGetUnsafe("Some", (checked_sub((termLen(op_4944[
          "prefix"
        ])), (left_branches * child_size)))))
      [] VariantTag((checked_sub((termLen(op_4944["prefix"])), (left_branches
        * child_size))))
        = "None"
        -> LET (*
          @type: (({ tag: Str }) => Bool);
        *)
        __QUINT_LAMBDA122(id__4937) == FALSE
        IN
        __QUINT_LAMBDA122(VariantGetUnsafe("None", (checked_sub((termLen(op_4944[
          "prefix"
        ])), (left_branches * child_size)))))

(*
  @type: (({ child_order: Seq(Int), child_size: Int, empty_child: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), max_prefix_length: Int, min_prefix_length: Int }, { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }) => Bool);
*)
right_branches_are_empty(spec_5027, op_5027) ==
  LET (*
    @type: (() => None({ tag: Str }) | Some(Int));
  *)
  idx == order_from_padding(spec_5027, op_5027)
  IN
  idx /= None
    /\ LET (*
      @type: (() => Int);
    *)
    right_branches == (Len(spec_5027["child_order"]) - 1) - unwrap((idx))
    IN
    IF right_branches = 0
    THEN FALSE
    ELSE IF termLen(op_5027["suffix"]) /= spec_5027["child_size"]
    THEN FALSE
    ELSE \A i_5020 \in 0 .. right_branches - 1:
      LET (*
        @type: (() => Int);
      *)
      idx_5019 ==
        unwrap((LET (*
          @type: ((Int) => Bool);
        *)
        __QUINT_LAMBDA125(x_4994) == x_4994 = i_5020
        IN
        findFirst(spec_5027["child_order"], __QUINT_LAMBDA125)))
      IN
      LET (*
        @type: (() => Int);
      *)
      from_index == idx_5019 * spec_5027["child_size"]
      IN
      spec_5027["empty_child"]
        = termSlice(op_5027["suffix"], (from_index), (from_index
          + spec_5027["child_size"]))

(*
  @type: (({ child_order: Seq(Int), child_size: Int, empty_child: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), max_prefix_length: Int, min_prefix_length: Int }, { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }) => Bool);
*)
is_left_step(spec_5060, left_5060, right_5060) ==
  LET (*
    @type: (() => None({ tag: Str }) | Some(Int));
  *)
  left_idx == order_from_padding(spec_5060, left_5060)
  IN
  LET (*
    @type: (() => None({ tag: Str }) | Some(Int));
  *)
  right_idx == order_from_padding(spec_5060, right_5060)
  IN
  left_idx /= None
    /\ right_idx /= None
    /\ unwrap((left_idx)) + 1 = unwrap((right_idx))

(*
  @type: (() => Bool);
*)
operationSuccessInv ==
  LET (*
    @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), { key_hash: Seq(Int), value_hash: Seq(Int) }) => Bool);
  *)
  treeContainsKV(t_886, n_886) == Leaf(n_886) \in values(t_886)
  IN
  LET (*
    @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), Seq(Int)) => Bool);
  *)
  treeNotContainsKey(t_915, key_915) ==
    Cardinality({
      node_910 \in values(t_915):
        CASE VariantTag(node_910) = "Leaf"
            -> LET (*
              @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => Bool);
            *)
            __QUINT_LAMBDA166(n_905) == n_905["key_hash"] = key_915
            IN
            __QUINT_LAMBDA166(VariantGetUnsafe("Leaf", node_910))
          [] VariantTag(node_910) = "Internal"
            -> LET (*
              @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => Bool);
            *)
            __QUINT_LAMBDA167(id__908) == FALSE
            IN
            __QUINT_LAMBDA167(VariantGetUnsafe("Internal", node_910))
    })
      = 0
  IN
  LET (*
    @type: (() => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
  *)
  tm == treeAtVersion(tree, (version - 1))
  IN
  Len(ops_history) > 0
    => (\A op_954 \in last(ops_history):
      CASE VariantTag(op_954["op"]) = "Insert"
          -> LET (*
            @type: ((Seq(Int)) => Bool);
          *)
          __QUINT_LAMBDA168(value_949) ==
            treeContainsKV((tm), [key_hash |-> op_954["key_hash"],
              value_hash |-> value_949])
          IN
          __QUINT_LAMBDA168(VariantGetUnsafe("Insert", op_954["op"]))
        [] VariantTag(op_954["op"]) = "Delete"
          -> LET (*
            @type: (({ tag: Str }) => Bool);
          *)
          __QUINT_LAMBDA169(id__952) ==
            treeNotContainsKey((tm), op_954["key_hash"])
          IN
          __QUINT_LAMBDA169(VariantGetUnsafe("Delete", op_954["op"])))

(*
  @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
hash(n_4019) ==
  CASE VariantTag(n_4019) = "Leaf"
      -> LET (*
        @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
      *)
      __QUINT_LAMBDA29(n_4014) == hashLeafNode(n_4014)
      IN
      __QUINT_LAMBDA29(VariantGetUnsafe("Leaf", n_4019))
    [] VariantTag(n_4019) = "Internal"
      -> LET (*
        @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
      *)
      __QUINT_LAMBDA30(n_4017) == hashInternalNode(n_4017)
      IN
      __QUINT_LAMBDA30(VariantGetUnsafe("Internal", n_4019))

(*
  @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
*)
fancy_hash(fancy_n_4019) ==
  CASE VariantTag(fancy_n_4019) = "Leaf"
      -> LET (*
        @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
      *)
      __QUINT_LAMBDA38(fancy_n_4014) == fancy_hashLeafNode(fancy_n_4014)
      IN
      __QUINT_LAMBDA38(VariantGetUnsafe("Leaf", fancy_n_4019))
    [] VariantTag(fancy_n_4019) = "Internal"
      -> LET (*
        @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
      *)
      __QUINT_LAMBDA39(fancy_n_4017) == fancy_hashInternalNode(fancy_n_4017)
      IN
      __QUINT_LAMBDA39(VariantGetUnsafe("Internal", fancy_n_4019))

(*
  @type: (({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }, (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), Seq(Int), Seq(Int)) => Bool);
*)
verify_existence(proof_4484, root_4484, key_4484, value_4484) ==
  key_4484 = proof_4484["key"]
    /\ value_4484 = proof_4484["value"]
    /\ root_4484 = exists_calculate(proof_4484)

(*
  @type: (({ child_order: Seq(Int), child_size: Int, empty_child: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), max_prefix_length: Int, min_prefix_length: Int }, Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) })) => Bool);
*)
is_left_most(spec_4751, path_4751) ==
  CASE VariantTag((get_padding(spec_4751, 0))) = "Some"
      -> LET (*
        @type: (({ max_prefix: Int, min_prefix: Int, suffix: Int }) => Bool);
      *)
      __QUINT_LAMBDA123(pad_4746) ==
        \A i_4741 \in LET (*
          @type: (() => Set(Int));
        *)
        __quint_var18 == DOMAIN path_4751
        IN
        IF __quint_var18 = {}
        THEN {}
        ELSE (__quint_var18 \union {0}) \ {(Len(path_4751))}:
          LET (*
            @type: (() => { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) });
          *)
          step == path_4751[(i_4741 + 1)]
          IN
          has_padding((step), pad_4746)
            \/ left_branches_are_empty(spec_4751, (step))
      IN
      __QUINT_LAMBDA123(VariantGetUnsafe("Some", (get_padding(spec_4751, 0))))
    [] VariantTag((get_padding(spec_4751, 0))) = "None"
      -> LET (*
        @type: (({ tag: Str }) => Bool);
      *)
      __QUINT_LAMBDA124(id__4749) == FALSE
      IN
      __QUINT_LAMBDA124(VariantGetUnsafe("None", (get_padding(spec_4751, 0))))

(*
  @type: (({ child_order: Seq(Int), child_size: Int, empty_child: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), max_prefix_length: Int, min_prefix_length: Int }, Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) })) => Bool);
*)
is_right_most(spec_4795, path_4795) ==
  LET (*@type: (() => Int); *) idx == Len(spec_4795["child_order"]) - 1 IN
  CASE VariantTag((get_padding(spec_4795, (idx)))) = "Some"
      -> LET (*
        @type: (({ max_prefix: Int, min_prefix: Int, suffix: Int }) => Bool);
      *)
      __QUINT_LAMBDA126(pad_4789) ==
        \A i_4784 \in LET (*
          @type: (() => Set(Int));
        *)
        __quint_var19 == DOMAIN path_4795
        IN
        IF __quint_var19 = {}
        THEN {}
        ELSE (__quint_var19 \union {0}) \ {(Len(path_4795))}:
          LET (*
            @type: (() => { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) });
          *)
          step == path_4795[(i_4784 + 1)]
          IN
          has_padding((step), pad_4789)
            \/ right_branches_are_empty(spec_4795, (step))
      IN
      __QUINT_LAMBDA126(VariantGetUnsafe("Some", (get_padding(spec_4795, (idx)))))
    [] VariantTag((get_padding(spec_4795, (idx)))) = "None"
      -> LET (*
        @type: (({ tag: Str }) => Bool);
      *)
      __QUINT_LAMBDA127(id__4792) == FALSE
      IN
      __QUINT_LAMBDA127(VariantGetUnsafe("None", (get_padding(spec_4795, (idx)))))

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, Int) => Bool);
*)
membershipSoundness(tree_7451, v_7451) ==
  LET (*
    @type: (() => Set(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
  *)
  nodes == values((treeAtVersion(tree_7451, v_7451)))
  IN
  \A k_7448 \in all_key_hashes:
    CASE VariantTag((ics23_prove(tree_7451, k_7448, v_7451))) = "Some"
        -> LET (*
          @type: ((Exist({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) | NonExist({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) })) => Bool);
        *)
        __QUINT_LAMBDA136(cp_7443) ==
          CASE VariantTag(cp_7443) = "Exist"
              -> LET (*
                @type: (({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) => Bool);
              *)
              __QUINT_LAMBDA134(ep_7435) ==
                Cardinality({
                  n_7427 \in nodes:
                    CASE VariantTag(n_7427) = "Leaf"
                        -> LET (*
                          @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => Bool);
                        *)
                        __QUINT_LAMBDA132(n_7422) == n_7422["key_hash"] = k_7448
                        IN
                        __QUINT_LAMBDA132(VariantGetUnsafe("Leaf", n_7427))
                      [] VariantTag(n_7427) = "Internal"
                        -> LET (*
                          @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => Bool);
                        *)
                        __QUINT_LAMBDA133(id__7425) == FALSE
                        IN
                        __QUINT_LAMBDA133(VariantGetUnsafe("Internal", n_7427))
                })
                  > 0
              IN
              __QUINT_LAMBDA134(VariantGetUnsafe("Exist", cp_7443))
            [] VariantTag(cp_7443) = "NonExist"
              -> LET (*
                @type: (({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) }) => Bool);
              *)
              __QUINT_LAMBDA135(id__7438) == TRUE
              IN
              __QUINT_LAMBDA135(VariantGetUnsafe("NonExist", cp_7443))
        IN
        __QUINT_LAMBDA136(VariantGetUnsafe("Some", (ics23_prove(tree_7451, k_7448,
        v_7451))))
      [] VariantTag((ics23_prove(tree_7451, k_7448, v_7451))) = "None"
        -> LET (*
          @type: (({ tag: Str }) => Bool);
        *)
        __QUINT_LAMBDA137(id__7446) == TRUE
        IN
        __QUINT_LAMBDA137(VariantGetUnsafe("None", (ics23_prove(tree_7451, k_7448,
        v_7451))))

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, Int) => Bool);
*)
nonMembershipSoundness(tree_7510, v_7510) ==
  LET (*
    @type: (() => Set(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
  *)
  nodes == values((treeAtVersion(tree_7510, v_7510)))
  IN
  \A k_7507 \in all_key_hashes:
    CASE VariantTag((ics23_prove(tree_7510, k_7507, v_7510))) = "Some"
        -> LET (*
          @type: ((Exist({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) | NonExist({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) })) => Bool);
        *)
        __QUINT_LAMBDA142(cp_7502) ==
          CASE VariantTag(cp_7502) = "NonExist"
              -> LET (*
                @type: (({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) }) => Bool);
              *)
              __QUINT_LAMBDA140(nep_7494) ==
                Cardinality({
                  n_7486 \in nodes:
                    CASE VariantTag(n_7486) = "Leaf"
                        -> LET (*
                          @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => Bool);
                        *)
                        __QUINT_LAMBDA138(n_7481) == n_7481["key_hash"] = k_7507
                        IN
                        __QUINT_LAMBDA138(VariantGetUnsafe("Leaf", n_7486))
                      [] VariantTag(n_7486) = "Internal"
                        -> LET (*
                          @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => Bool);
                        *)
                        __QUINT_LAMBDA139(id__7484) == FALSE
                        IN
                        __QUINT_LAMBDA139(VariantGetUnsafe("Internal", n_7486))
                })
                  = 0
              IN
              __QUINT_LAMBDA140(VariantGetUnsafe("NonExist", cp_7502))
            [] VariantTag(cp_7502) = "Exist"
              -> LET (*
                @type: (({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) => Bool);
              *)
              __QUINT_LAMBDA141(id__7497) == TRUE
              IN
              __QUINT_LAMBDA141(VariantGetUnsafe("Exist", cp_7502))
        IN
        __QUINT_LAMBDA142(VariantGetUnsafe("Some", (ics23_prove(tree_7510, k_7507,
        v_7510))))
      [] VariantTag((ics23_prove(tree_7510, k_7507, v_7510))) = "None"
        -> LET (*
          @type: (({ tag: Str }) => Bool);
        *)
        __QUINT_LAMBDA143(id__7505) == TRUE
        IN
        __QUINT_LAMBDA143(VariantGetUnsafe("None", (ics23_prove(tree_7510, k_7507,
        v_7510))))

(*
  @type: (() => Bool);
*)
everyNodesParentIsInTheTreeInv ==
  LET (*
    @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Bool);
  *)
  everyNodesParentIsInTheTree(t_246) ==
    LET (*
      @type: (() => Set(Seq(Int)));
    *)
    prefixes == { p_220["key_hash"]: p_220 \in DOMAIN t_246 }
    IN
    \A p_243 \in { p_228 \in prefixes: p_228 /= <<>> }:
      LET (*
        @type: (() => Seq(Int));
      *)
      parent == SubSeq(p_243, (0 + 1), (Len(p_243) - 1))
      IN
      parent \in prefixes
  IN
  \A __quint_var22 \in treesToCheck: everyNodesParentIsInTheTree(__quint_var22)

(*
  @type: (() => Bool);
*)
nodeAtCommonPrefixInv ==
  LET (*
    @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), Seq(Int)) => Bool);
  *)
  existsNode(t_272, b_272) ==
    Cardinality({ nId_267 \in DOMAIN t_272: nId_267["key_hash"] = b_272 }) > 0
  IN
  LET (*
    @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Bool);
  *)
  nodeAtCommonPrefix(t_300) ==
    \A a_298 \in allLeafs(t_300):
      \A b_296 \in allLeafs(t_300):
        a_298["key_hash"] /= b_296["key_hash"]
          => existsNode(t_300, (commonPrefix(a_298, b_296)))
  IN
  \A __quint_var24 \in treesToCheck: nodeAtCommonPrefix(__quint_var24)

(*
  @type: (() => Bool);
*)
noLeafInPrefixesInv ==
  LET (*
    @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Bool);
  *)
  noLeafInPrefixes(t_355) ==
    LET (*
      @type: (() => Set(Seq(Int)));
    *)
    nodes == { nId_318["key_hash"]: nId_318 \in DOMAIN t_355 }
    IN
    LET (*
      @type: (() => Set(Seq(Int)));
    *)
    leafs ==
      {
        nId_334["key_hash"]:
          nId_334 \in { nId_328 \in DOMAIN t_355: isLeaf(t_355[nId_328]) }
      }
    IN
    \A node_351 \in nodes:
      ~(\E leaf_348 \in leafs:
        node_351 /= leaf_348 /\ prefix_of(leaf_348, node_351))
  IN
  \A __quint_var26 \in treesToCheck: noLeafInPrefixes(__quint_var26)

(*
  @type: (() => Bool);
*)
allInternalNodesHaveAChildInv ==
  LET (*
    @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => Bool);
  *)
  internalNodeHasAChild(n_377) ==
    n_377["left_child"] /= None \/ n_377["right_child"] /= None
  IN
  LET (*
    @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Bool);
  *)
  allInternalNodesHaveAChild(t_400) ==
    \A nId_398 \in DOMAIN t_400:
      CASE VariantTag(t_400[nId_398]) = "Internal"
          -> LET (*
            @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => Bool);
          *)
          __QUINT_LAMBDA150(n_393) == internalNodeHasAChild(n_393)
          IN
          __QUINT_LAMBDA150(VariantGetUnsafe("Internal", t_400[nId_398]))
        [] VariantTag(t_400[nId_398]) = "Leaf"
          -> LET (*
            @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => Bool);
          *)
          __QUINT_LAMBDA151(id__396) == TRUE
          IN
          __QUINT_LAMBDA151(VariantGetUnsafe("Leaf", t_400[nId_398]))
  IN
  \A __quint_var27 \in treesToCheck: allInternalNodesHaveAChild(__quint_var27)

(*
  @type: (() => Bool);
*)
densityInv ==
  LET (*
    @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Bool);
  *)
  isDense(t_469) ==
    \A nId_467 \in DOMAIN t_469:
      CASE VariantTag(t_469[nId_467]) = "Internal"
          -> LET (*
            @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => Bool);
          *)
          __QUINT_LAMBDA152(n_462) ==
            IF n_462["left_child"] = None /\ n_462["right_child"] /= None
            THEN isInternal((findNode(t_469, (Append(nId_467["key_hash"], 1)))))
            ELSE IF n_462["right_child"] = None /\ n_462["left_child"] /= None
            THEN isInternal((findNode(t_469, (Append(nId_467["key_hash"], 0)))))
            ELSE TRUE
          IN
          __QUINT_LAMBDA152(VariantGetUnsafe("Internal", t_469[nId_467]))
        [] VariantTag(t_469[nId_467]) = "Leaf"
          -> LET (*
            @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => Bool);
          *)
          __QUINT_LAMBDA153(id__465) == TRUE
          IN
          __QUINT_LAMBDA153(VariantGetUnsafe("Leaf", t_469[nId_467]))
  IN
  \A __quint_var28 \in treesToCheck: isDense(__quint_var28)

(*
  @type: (() => Bool);
*)
denseVersionsInv ==
  LET (*
    @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Bool);
  *)
  denseVersions(t_625) ==
    \A nId_623 \in DOMAIN t_625:
      CASE VariantTag(t_625[nId_623]) = "Internal"
          -> LET (*
            @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => Bool);
          *)
          __QUINT_LAMBDA158(n_618) ==
            LET (*
              @type: (() => Bool);
            *)
            leftOK ==
              CASE VariantTag(n_618["left_child"]) = "Some"
                  -> LET (*
                    @type: (({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) => Bool);
                  *)
                  __QUINT_LAMBDA154(c_570) ==
                    \E a_565 \in DOMAIN t_625:
                      a_565["key_hash"] = Append(nId_623["key_hash"], 0)
                        /\ a_565["version"] = nId_623["version"]
                  IN
                  __QUINT_LAMBDA154(VariantGetUnsafe("Some", n_618["left_child"]))
                [] VariantTag(n_618["left_child"]) = "None"
                  -> LET (*
                    @type: (({ tag: Str }) => Bool);
                  *)
                  __QUINT_LAMBDA155(id__573) == FALSE
                  IN
                  __QUINT_LAMBDA155(VariantGetUnsafe("None", n_618["left_child"]))
            IN
            LET (*
              @type: (() => Bool);
            *)
            rightOK ==
              CASE VariantTag(n_618["right_child"]) = "Some"
                  -> LET (*
                    @type: (({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) => Bool);
                  *)
                  __QUINT_LAMBDA156(c_604) ==
                    \E a_599 \in DOMAIN t_625:
                      a_599["key_hash"] = Append(nId_623["key_hash"], 1)
                        /\ a_599["version"] = nId_623["version"]
                  IN
                  __QUINT_LAMBDA156(VariantGetUnsafe("Some", n_618[
                    "right_child"
                  ]))
                [] VariantTag(n_618["right_child"]) = "None"
                  -> LET (*
                    @type: (({ tag: Str }) => Bool);
                  *)
                  __QUINT_LAMBDA157(id__607) == FALSE
                  IN
                  __QUINT_LAMBDA157(VariantGetUnsafe("None", n_618[
                    "right_child"
                  ]))
            IN
            leftOK \/ rightOK
          IN
          __QUINT_LAMBDA158(VariantGetUnsafe("Internal", t_625[nId_623]))
        [] VariantTag(t_625[nId_623]) = "Leaf"
          -> LET (*
            @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => Bool);
          *)
          __QUINT_LAMBDA159(id__621) == TRUE
          IN
          __QUINT_LAMBDA159(VariantGetUnsafe("Leaf", t_625[nId_623]))
  IN
  \A __quint_var29 \in treesToCheck: denseVersions(__quint_var29)

(*
  @type: (() => Bool);
*)
goodTreeMapInv ==
  LET (*
    @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Bool);
  *)
  goodTreeMap(t_850) ==
    \A a_848 \in DOMAIN t_850:
      \A b_846 \in DOMAIN t_850:
        a_848["key_hash"] = b_846["key_hash"]
          => a_848["version"] = b_846["version"]
  IN
  \A __quint_var31 \in treesToCheck: goodTreeMap(__quint_var31)

(*
  @type: (() => Bool);
*)
bijectiveTreeMapInv ==
  LET (*
    @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Bool);
  *)
  bijectiveTreeMap(t_868) ==
    Cardinality((DOMAIN t_868)) = Cardinality((values(t_868)))
  IN
  \A __quint_var32 \in treesToCheck: bijectiveTreeMap(__quint_var32)

(*
  @type: (() => Bool);
*)
uniqueHashesInv ==
  LET (*
    @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Bool);
  *)
  uniqueHashes(t_814) ==
    LET (*
      @type: (() => Seq((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))));
    *)
    hashes ==
      LET (*
        @type: ((Seq((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))), Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => Seq((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))));
      *)
      __QUINT_LAMBDA266(acc_793, node_793) ==
        CASE VariantTag(node_793) = "Internal"
            -> LET (*
              @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => Seq((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))));
            *)
            __QUINT_LAMBDA264(n_788) ==
              LET (*
                @type: (() => Seq((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))));
              *)
              acc_1 ==
                CASE VariantTag(n_788["left_child"]) = "None"
                    -> LET (*
                      @type: (({ tag: Str }) => Seq((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))));
                    *)
                    __QUINT_LAMBDA260(id__762) == acc_793
                    IN
                    __QUINT_LAMBDA260(VariantGetUnsafe("None", n_788[
                      "left_child"
                    ]))
                  [] VariantTag(n_788["left_child"]) = "Some"
                    -> LET (*
                      @type: (({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) => Seq((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))));
                    *)
                    __QUINT_LAMBDA261(c_765) == Append(acc_793, c_765["hash"])
                    IN
                    __QUINT_LAMBDA261(VariantGetUnsafe("Some", n_788[
                      "left_child"
                    ]))
              IN
              CASE VariantTag(n_788["right_child"]) = "None"
                  -> LET (*
                    @type: (({ tag: Str }) => Seq((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))));
                  *)
                  __QUINT_LAMBDA262(id__779) == acc_1
                  IN
                  __QUINT_LAMBDA262(VariantGetUnsafe("None", n_788[
                    "right_child"
                  ]))
                [] VariantTag(n_788["right_child"]) = "Some"
                  -> LET (*
                    @type: (({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) => Seq((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))));
                  *)
                  __QUINT_LAMBDA263(c_782) == Append((acc_1), c_782["hash"])
                  IN
                  __QUINT_LAMBDA263(VariantGetUnsafe("Some", n_788[
                    "right_child"
                  ]))
            IN
            __QUINT_LAMBDA264(VariantGetUnsafe("Internal", node_793))
          [] OTHER
            -> LET (*
              @type: ((a84) => Seq((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))));
            *)
            __QUINT_LAMBDA265(id__791) == acc_793
            IN
            __QUINT_LAMBDA265({})
      IN
      ApaFoldSet(__QUINT_LAMBDA266, <<>>, (values(t_814)))
    IN
    LET (*
      @type: (() => Set((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))));
    *)
    uniqueHashes_812 ==
      LET (*
        @type: ((Set((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))), (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))) => Set((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int)))));
      *)
      __QUINT_LAMBDA267(acc_804, hash_804) == acc_804 \union {hash_804}
      IN
      ApaFoldSeqLeft(__QUINT_LAMBDA267, {}, (hashes))
    IN
    Len((hashes)) = Cardinality((uniqueHashes_812))
  IN
  \A __quint_var54 \in treesToCheck: uniqueHashes(__quint_var54)

(*
  @type: ((Int, Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }));
*)
into_child(version_3846, outcome_3846) ==
  CASE VariantTag(outcome_3846) = "Updated"
      -> LET (*
        @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }));
      *)
      __QUINT_LAMBDA40(node_3838) ==
        Some([version |-> version_3846, hash |-> hash(node_3838)])
      IN
      __QUINT_LAMBDA40(VariantGetUnsafe("Updated", outcome_3846))
    [] VariantTag(outcome_3846) = "Unchanged"
      -> LET (*
        @type: ((None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }));
      *)
      __QUINT_LAMBDA41(id__3841) == None
      IN
      __QUINT_LAMBDA41(VariantGetUnsafe("Unchanged", outcome_3846))
    [] VariantTag(outcome_3846) = "Deleted"
      -> LET (*
        @type: (({ tag: Str }) => None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }));
      *)
      __QUINT_LAMBDA42(id__3844) == None
      IN
      __QUINT_LAMBDA42(VariantGetUnsafe("Deleted", outcome_3846))

(*
  @type: (((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), { key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }, Seq(Int), Seq(Int)) => Bool);
*)
verifyMembership(root_4463, proof_4463, key_4463, value_4463) ==
  verify_existence(proof_4463, root_4463, key_4463, value_4463)

(*
  @type: (({ child_order: Seq(Int), child_size: Int, empty_child: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), max_prefix_length: Int, min_prefix_length: Int }, Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) })) => Bool);
*)
is_left_neighbor(spec_5154, left_5154, right_5154) ==
  \E li_5152 \in LET (*
    @type: (() => Set(Int));
  *)
  __quint_var20 == DOMAIN left_5154
  IN
  IF __quint_var20 = {}
  THEN {}
  ELSE (__quint_var20 \union {0}) \ {(Len(left_5154))}:
    \E ri_5150 \in LET (*
      @type: (() => Set(Int));
    *)
    __quint_var21 == DOMAIN right_5154
    IN
    IF __quint_var21 = {}
    THEN {}
    ELSE (__quint_var21 \union {0}) \ {(Len(right_5154))}:
      Len(left_5154) - li_5152 = Len(right_5154) - ri_5150
        /\ LET (*
          @type: (() => Int);
        *)
        dist == (Len(left_5154) - 1) - li_5152
        IN
        \A k_5126 \in 1 .. dist:
          LET (*
            @type: (() => { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) });
          *)
          lnode == left_5154[((li_5152 + k_5126) + 1)]
          IN
          LET (*
            @type: (() => { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) });
          *)
          rnode == right_5154[((ri_5150 + k_5126) + 1)]
          IN
          (lnode)["prefix"] = (rnode)["prefix"]
            /\ (lnode)["suffix"] = (rnode)["suffix"]
        /\ is_left_step(spec_5154, left_5154[(li_5152 + 1)], right_5154[
          (ri_5150 + 1)
        ])
        /\ is_right_most(spec_5154, (SubSeq(left_5154, (0 + 1), li_5152)))
        /\ is_left_most(spec_5154, (SubSeq(right_5154, (0 + 1), ri_5150)))

(*
  @type: (() => Bool);
*)
membershipSoundnessInv ==
  \A v_982 \in versionsToCheck: membershipSoundness(tree, v_982)

(*
  @type: (() => Bool);
*)
nonMembershipSoundnessInv ==
  \A v_990 \in versionsToCheck: nonMembershipSoundness(tree, v_990)

(*
  @type: (() => Bool);
*)
hashInv ==
  LET (*
    @type: ((({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Bool);
  *)
  properlyHashed(t_734) ==
    \A nID_732 \in DOMAIN t_734:
      CASE VariantTag(t_734[nID_732]) = "Leaf"
          -> LET (*
            @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => Bool);
          *)
          __QUINT_LAMBDA160(id__727) == TRUE
          IN
          __QUINT_LAMBDA160(VariantGetUnsafe("Leaf", t_734[nID_732]))
        [] VariantTag(t_734[nID_732]) = "Internal"
          -> LET (*
            @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => Bool);
          *)
          __QUINT_LAMBDA165(n_730) ==
            (CASE VariantTag(n_730["left_child"]) = "None"
                  -> LET (*
                    @type: (({ tag: Str }) => Bool);
                  *)
                  __QUINT_LAMBDA161(id__696) == TRUE
                  IN
                  __QUINT_LAMBDA161(VariantGetUnsafe("None", n_730["left_child"]))
                [] VariantTag(n_730["left_child"]) = "Some"
                  -> LET (*
                    @type: (({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) => Bool);
                  *)
                  __QUINT_LAMBDA162(c_699) ==
                    c_699["hash"]
                      = hash((findNode(t_734, (Append(nID_732["key_hash"], 0)))))
                  IN
                  __QUINT_LAMBDA162(VariantGetUnsafe("Some", n_730["left_child"])))
              /\ (CASE VariantTag(n_730["right_child"]) = "None"
                  -> LET (*
                    @type: (({ tag: Str }) => Bool);
                  *)
                  __QUINT_LAMBDA163(id__719) == TRUE
                  IN
                  __QUINT_LAMBDA163(VariantGetUnsafe("None", n_730[
                    "right_child"
                  ]))
                [] VariantTag(n_730["right_child"]) = "Some"
                  -> LET (*
                    @type: (({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) => Bool);
                  *)
                  __QUINT_LAMBDA164(c_722) ==
                    c_722["hash"]
                      = hash((findNode(t_734, (Append(nID_732["key_hash"], 1)))))
                  IN
                  __QUINT_LAMBDA164(VariantGetUnsafe("Some", n_730[
                    "right_child"
                  ])))
          IN
          __QUINT_LAMBDA165(VariantGetUnsafe("Internal", t_734[nID_732]))
  IN
  \A __quint_var30 \in treesToCheck: properlyHashed(__quint_var30)

(*
  @type: ((Int, Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }));
*)
fancy_into_child(fancy_version_3846, fancy_outcome_3846) ==
  CASE VariantTag(fancy_outcome_3846) = "Updated"
      -> LET (*
        @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }));
      *)
      __QUINT_LAMBDA186(fancy_node_3838) ==
        fancy_Some([version |-> fancy_version_3846,
          hash |-> fancy_hash(fancy_node_3838)])
      IN
      __QUINT_LAMBDA186(VariantGetUnsafe("Updated", fancy_outcome_3846))
    [] VariantTag(fancy_outcome_3846) = "Unchanged"
      -> LET (*
        @type: ((None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }));
      *)
      __QUINT_LAMBDA187(fancy___3841) == fancy_None
      IN
      __QUINT_LAMBDA187(VariantGetUnsafe("Unchanged", fancy_outcome_3846))
    [] VariantTag(fancy_outcome_3846) = "Deleted"
      -> LET (*
        @type: (({ tag: Str }) => None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }));
      *)
      __QUINT_LAMBDA188(fancy___3844) == fancy_None
      IN
      __QUINT_LAMBDA188(VariantGetUnsafe("Deleted", fancy_outcome_3846))

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, Int) => Bool);
*)
membershipCompleteness(tree_7315, v_7315) ==
  has(tree_7315["nodes"], [key_hash |-> ROOT_BITS, version |-> v_7315])
    => (\A node_7312 \in values((treeAtVersion(tree_7315, v_7315))):
      CASE VariantTag(node_7312) = "Leaf"
          -> LET (*
            @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => Bool);
          *)
          __QUINT_LAMBDA109(n_7307) ==
            CASE VariantTag((ics23_prove(tree_7315, n_7307["key_hash"], v_7315)))
                = "Some"
                -> LET (*
                  @type: ((Exist({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) | NonExist({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) })) => Bool);
                *)
                __QUINT_LAMBDA107(cp_7299) ==
                  CASE VariantTag(cp_7299) = "Exist"
                      -> LET (*
                        @type: (({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) => Bool);
                      *)
                      __QUINT_LAMBDA105(ep_7291) ==
                        LET (*
                          @type: (() => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
                        *)
                        root ==
                          hash(tree_7315["nodes"][
                            [key_hash |-> ROOT_BITS, version |-> v_7315]
                          ])
                        IN
                        verifyMembership((root), ep_7291, n_7307["key_hash"], n_7307[
                          "value_hash"
                        ])
                      IN
                      __QUINT_LAMBDA105(VariantGetUnsafe("Exist", cp_7299))
                    [] VariantTag(cp_7299) = "NonExist"
                      -> LET (*
                        @type: (({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) }) => Bool);
                      *)
                      __QUINT_LAMBDA106(id__7294) == FALSE
                      IN
                      __QUINT_LAMBDA106(VariantGetUnsafe("NonExist", cp_7299))
                IN
                __QUINT_LAMBDA107(VariantGetUnsafe("Some", (ics23_prove(tree_7315,
                n_7307["key_hash"], v_7315))))
              [] VariantTag((ics23_prove(tree_7315, n_7307["key_hash"], v_7315)))
                = "None"
                -> LET (*
                  @type: (({ tag: Str }) => Bool);
                *)
                __QUINT_LAMBDA108(id__7302) == FALSE
                IN
                __QUINT_LAMBDA108(VariantGetUnsafe("None", (ics23_prove(tree_7315,
                n_7307["key_hash"], v_7315))))
          IN
          __QUINT_LAMBDA109(VariantGetUnsafe("Leaf", node_7312))
        [] VariantTag(node_7312) = "Internal"
          -> LET (*
            @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => Bool);
          *)
          __QUINT_LAMBDA110(id__7310) == TRUE
          IN
          __QUINT_LAMBDA110(VariantGetUnsafe("Internal", node_7312)))

(*
  @type: (({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) }, { child_order: Seq(Int), child_size: Int, empty_child: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), max_prefix_length: Int, min_prefix_length: Int }, (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), Seq(Int)) => Bool);
*)
verify_non_existence(proof_4652, spec_4652, root_4652, key_4652) ==
  (proof_4652["left"] /= None \/ proof_4652["right"] /= None)
    /\ (proof_4652["left"] /= None
      => LET (*
        @type: (() => { key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) });
      *)
      left == unwrap(proof_4652["left"])
      IN
      verify_existence((left), root_4652, (left)["key"], (left)["value"])
        /\ greater_than(key_4652, (left)["key"]))
    /\ (proof_4652["right"] /= None
      => LET (*
        @type: (() => { key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) });
      *)
      right == unwrap(proof_4652["right"])
      IN
      verify_existence((right), root_4652, (right)["key"], (right)["value"])
        /\ less_than(key_4652, (right)["key"]))
    /\ (IF proof_4652["left"] = None
    THEN is_left_most(spec_4652, (unwrap(proof_4652["right"]))["path"])
    ELSE IF proof_4652["right"] = None
    THEN is_right_most(spec_4652, (unwrap(proof_4652["left"]))["path"])
    ELSE is_left_neighbor(spec_4652, (unwrap(proof_4652["left"]))["path"], (unwrap(proof_4652[
      "right"
    ]))[
      "path"
    ]))

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, (<<Int, Seq(Int), Set({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) })>> -> { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) }), Int, Seq(Int), Set({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) })) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
*)
fancy_create_subtree_with_memo(fancy_tree_7233, fancy_memo_7233, fancy_version_7233,
fancy_bits_7233, fancy_batch_7233, fancy_existing_leaf_7233) ==
  IF Cardinality(fancy_batch_7233) = 0 /\ fancy_existing_leaf_7233 = fancy_None
  THEN [outcome |-> fancy_Unchanged((fancy_None)), nodes_to_add |-> {}]
  ELSE IF Cardinality(fancy_batch_7233) = 0
  THEN LET (*
    @type: (() => Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }));
  *)
  fancy_node == fancy_Leaf((fancy_unwrap(fancy_existing_leaf_7233)))
  IN
  [outcome |-> fancy_Updated((fancy_node)), nodes_to_add |-> {}]
  ELSE IF Cardinality(fancy_batch_7233) = 1
    /\ fancy_existing_leaf_7233 = fancy_None
  THEN LET (*
    @type: (() => Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }));
  *)
  fancy_node == fancy_Leaf((getOnlyElement(fancy_batch_7233)))
  IN
  [outcome |-> fancy_Updated((fancy_node)), nodes_to_add |-> {}]
  ELSE LET (*
    @type: (() => <<Set({ key_hash: Seq(Int), value_hash: Seq(Int) }), Set({ key_hash: Seq(Int), value_hash: Seq(Int) })>>);
  *)
  fancy_partitioned_batch ==
    fancy_partition_batch(fancy_batch_7233, fancy_bits_7233)
  IN
  LET (*
    @type: (() => Set({ key_hash: Seq(Int), value_hash: Seq(Int) }));
  *)
  fancy_batch_for_left == (fancy_partitioned_batch)[1]
  IN
  LET (*
    @type: (() => Set({ key_hash: Seq(Int), value_hash: Seq(Int) }));
  *)
  fancy_batch_for_right == (fancy_partitioned_batch)[2]
  IN
  LET (*
    @type: (() => <<None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) })>>);
  *)
  fancy_partitioned_leaf ==
    fancy_partition_leaf(fancy_existing_leaf_7233, fancy_bits_7233)
  IN
  LET (*
    @type: (() => None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) }));
  *)
  fancy_leaf_for_left == (fancy_partitioned_leaf)[1]
  IN
  LET (*
    @type: (() => None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) }));
  *)
  fancy_leaf_for_right == (fancy_partitioned_leaf)[2]
  IN
  LET (*
    @type: (() => Seq(Int));
  *)
  fancy_left_bits == Append(fancy_bits_7233, 0)
  IN
  LET (*
    @type: (() => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
  *)
  fancy_left ==
    fancy_memo_7233[
      <<
        fancy_version_7233, (fancy_left_bits), (fancy_batch_for_left), (fancy_leaf_for_left)
      >>
    ]
  IN
  LET (*
    @type: (() => Seq(Int));
  *)
  fancy_right_bits == Append(fancy_bits_7233, 1)
  IN
  LET (*
    @type: (() => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
  *)
  fancy_right ==
    fancy_memo_7233[
      <<
        fancy_version_7233, (fancy_right_bits), (fancy_batch_for_right), (fancy_leaf_for_right)
      >>
    ]
  IN
  LET (*
    @type: (() => Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>));
  *)
  fancy_nodes_to_add ==
    (((fancy_left)["nodes_to_add"] \union (fancy_right)["nodes_to_add"])
      \union (CASE VariantTag((fancy_left)["outcome"]) = "Updated"
          -> LET (*
            @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>));
          *)
          __QUINT_LAMBDA189(fancy_node_7147) ==
            {<<
              [version |-> fancy_version_7233, key_hash |-> fancy_left_bits], fancy_node_7147
            >>}
          IN
          __QUINT_LAMBDA189(VariantGetUnsafe("Updated", (fancy_left)["outcome"]))
        [] VariantTag((fancy_left)["outcome"]) = "Unchanged"
          -> LET (*
            @type: ((None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>));
          *)
          __QUINT_LAMBDA192(fancy_option_7150) ==
            CASE VariantTag(fancy_option_7150) = "Some"
                -> LET (*
                  @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>));
                *)
                __QUINT_LAMBDA190(fancy_node_7139) ==
                  {<<
                    [version |-> fancy_version_7233,
                      key_hash |-> fancy_left_bits], fancy_node_7139
                  >>}
                IN
                __QUINT_LAMBDA190(VariantGetUnsafe("Some", fancy_option_7150))
              [] OTHER
                -> LET (*
                  @type: ((a60) => Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>));
                *)
                __QUINT_LAMBDA191(fancy___7142) == {}
                IN
                __QUINT_LAMBDA191({})
          IN
          __QUINT_LAMBDA192(VariantGetUnsafe("Unchanged", (fancy_left)[
            "outcome"
          ]))
        [] OTHER
          -> LET (*
            @type: ((a61) => Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>));
          *)
          __QUINT_LAMBDA193(fancy___7153) == {}
          IN
          __QUINT_LAMBDA193({})))
      \union (CASE VariantTag((fancy_right)["outcome"]) = "Updated"
          -> LET (*
            @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>));
          *)
          __QUINT_LAMBDA194(fancy_node_7187) ==
            {<<
              [version |-> fancy_version_7233, key_hash |-> fancy_right_bits], fancy_node_7187
            >>}
          IN
          __QUINT_LAMBDA194(VariantGetUnsafe("Updated", (fancy_right)["outcome"]))
        [] VariantTag((fancy_right)["outcome"]) = "Unchanged"
          -> LET (*
            @type: ((None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>));
          *)
          __QUINT_LAMBDA197(fancy_option_7190) ==
            CASE VariantTag(fancy_option_7190) = "Some"
                -> LET (*
                  @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>));
                *)
                __QUINT_LAMBDA195(fancy_node_7179) ==
                  {<<
                    [version |-> fancy_version_7233,
                      key_hash |-> fancy_right_bits], fancy_node_7179
                  >>}
                IN
                __QUINT_LAMBDA195(VariantGetUnsafe("Some", fancy_option_7190))
              [] OTHER
                -> LET (*
                  @type: ((a62) => Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>));
                *)
                __QUINT_LAMBDA196(fancy___7182) == {}
                IN
                __QUINT_LAMBDA196({})
          IN
          __QUINT_LAMBDA197(VariantGetUnsafe("Unchanged", (fancy_right)[
            "outcome"
          ]))
        [] OTHER
          -> LET (*
            @type: ((a63) => Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>));
          *)
          __QUINT_LAMBDA198(fancy___7193) == {}
          IN
          __QUINT_LAMBDA198({}))
  IN
  LET (*
    @type: (() => Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }));
  *)
  fancy_node ==
    fancy_Internal([left_child |->
        fancy_into_child(fancy_version_7233, (fancy_left)["outcome"]),
      right_child |->
        fancy_into_child(fancy_version_7233, (fancy_right)["outcome"])])
  IN
  [outcome |-> fancy_Updated((fancy_node)), nodes_to_add |-> fancy_nodes_to_add]

(*
  @type: (() => Bool);
*)
treeInvariants ==
  (IF everyNodesParentIsInTheTreeInv
    THEN TRUE
    ELSE q_debug("everyNodesParentIsInTheTreeInv", FALSE))
    /\ (IF nodeAtCommonPrefixInv
    THEN TRUE
    ELSE q_debug("nodeAtCommonPrefixInv", FALSE))
    /\ (IF noLeafInPrefixesInv
    THEN TRUE
    ELSE q_debug("noLeafInPrefixesInv", FALSE))
    /\ (IF allInternalNodesHaveAChildInv
    THEN TRUE
    ELSE q_debug("allInternalNodesHaveAChild", FALSE))
    /\ (IF densityInv THEN TRUE ELSE q_debug("densityInv", FALSE))
    /\ (IF versionInv THEN TRUE ELSE q_debug("versionInv", FALSE))
    /\ (IF orphansInNoTreeInv
    THEN TRUE
    ELSE q_debug("orphansInNoTreeInv", FALSE))
    /\ (IF hashInv THEN TRUE ELSE q_debug("hashInv", FALSE))
    /\ (IF uniqueHashesInv THEN TRUE ELSE q_debug("uniqueHashesInv", FALSE))
    /\ (IF goodTreeMapInv THEN TRUE ELSE q_debug("goodTreeMapInv", FALSE))
    /\ (IF bijectiveTreeMapInv
    THEN TRUE
    ELSE q_debug("bijectiveTreeMapInv", FALSE))
    /\ (IF operationSuccessInv
    THEN TRUE
    ELSE q_debug("operationSuccessInv", FALSE))

(*
  @type: (() => Bool);
*)
membershipCompletenessInv ==
  \A v_966 \in versionsToCheck: membershipCompleteness(tree, v_966)

(*
  @type: (((Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), { key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) }, Seq(Int)) => Bool);
*)
verifyNonMembership(root_4533, np_4533, key_4533) ==
  verify_non_existence(np_4533, (ics23_InnerSpec), root_4533, key_4533)

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, Int, Seq(Int), Set({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) })) => (<<Int, Seq(Int), Set({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) })>> -> { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) }));
*)
fancy_pre_compute_create_subtree(fancy_tree_6990, fancy_version_6990, fancy_bits_6990,
fancy_batch_6990, fancy_existing_leaf_6990) ==
  LET (*
    @type: (() => Seq(Seq(Int)));
  *)
  fancy_bits_to_compute ==
    LET (*
      @type: ((Seq(Int), Seq(Int)) => EQ({ tag: Str }) | GT({ tag: Str }) | LT({ tag: Str }));
    *)
    __QUINT_LAMBDA203(fancy_a_6934, fancy_b_6934) ==
      fancy_intCompare((Len(fancy_a_6934)), (Len(fancy_b_6934)))
    IN
    fancy_toList({
      fancy_b_6925 \in allListsUpTo({ 0, 1 }, (fancy_MAX_HASH_LENGTH)):
        fancy_isPrefixOf(fancy_bits_6990, fancy_b_6925)
    }, __QUINT_LAMBDA203)
  IN
  LET (*
    @type: ((Seq(Int), (<<Int, Seq(Int), Set({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) })>> -> { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) })) => (<<Int, Seq(Int), Set({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) })>> -> { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) }));
  *)
  __QUINT_LAMBDA204(fancy_bits_now_6987, fancy_memo_6987) ==
    LET (*
      @type: (() => Set({ key_hash: Seq(Int), value_hash: Seq(Int) }));
    *)
    fancy_batch_now ==
      {
        fancy_kv_6948 \in fancy_batch_6990:
          fancy_isPrefixOf(fancy_bits_now_6987, fancy_kv_6948["key_hash"])
      }
    IN
    LET (*
      @type: (() => None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) }));
    *)
    fancy_existing_leaf_now ==
      IF fancy_existing_leaf_6990 /= fancy_None
        /\ fancy_isPrefixOf(fancy_bits_now_6987, (fancy_unwrap(fancy_existing_leaf_6990))[
          "key_hash"
        ])
      THEN fancy_existing_leaf_6990
      ELSE fancy_None
    IN
    LET (*
      @type: (() => <<Int, Seq(Int), Set({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) })>>);
    *)
    fancy_memo_key ==
      <<
        fancy_version_6990, fancy_bits_now_6987, (fancy_batch_now), (fancy_existing_leaf_now)
      >>
    IN
    LET (*
      @type: (() => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
    *)
    fancy_memo_value ==
      fancy_create_subtree_with_memo(fancy_tree_6990, fancy_memo_6987, fancy_version_6990,
      fancy_bits_now_6987, (fancy_batch_now), (fancy_existing_leaf_now))
    IN
    LET (*
      @type: (() => (<<Int, Seq(Int), Set({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) })>> -> { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) }));
    *)
    __quint_var36 == fancy_memo_6987
    IN
    LET (*
      @type: (() => Set(<<Int, Seq(Int), Set({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) })>>));
    *)
    __quint_var37 == DOMAIN __quint_var36
    IN
    [
      __quint_var38 \in {(fancy_memo_key)} \union __quint_var37 |->
        IF __quint_var38 = fancy_memo_key
        THEN fancy_memo_value
        ELSE (__quint_var36)[__quint_var38]
    ]
  IN
  foldr((fancy_bits_to_compute), SetAsFun({}), __QUINT_LAMBDA204)

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, Int) => Bool);
*)
nonMembershipCompleteness(tree_7387, v_7387) ==
  has(tree_7387["nodes"], [key_hash |-> ROOT_BITS, version |-> v_7387])
    => LET (*
      @type: (() => Set(Seq(Int)));
    *)
    key_hashes_from_tree ==
      {
        l_7339["key_hash"]:
          l_7339 \in allLeafs((treeAtVersion(tree_7387, v_7387)))
      }
    IN
    LET (*
      @type: (() => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
    *)
    root ==
      hash(tree_7387["nodes"][[key_hash |-> ROOT_BITS, version |-> v_7387]])
    IN
    \A k_7382 \in all_key_hashes \ key_hashes_from_tree:
      CASE VariantTag((ics23_prove(tree_7387, k_7382, v_7387))) = "Some"
          -> LET (*
            @type: ((Exist({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) | NonExist({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) })) => Bool);
          *)
          __QUINT_LAMBDA130(cp_7377) ==
            CASE VariantTag(cp_7377) = "NonExist"
                -> LET (*
                  @type: (({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) }) => Bool);
                *)
                __QUINT_LAMBDA128(nep_7369) ==
                  verifyNonMembership((root), nep_7369, k_7382)
                IN
                __QUINT_LAMBDA128(VariantGetUnsafe("NonExist", cp_7377))
              [] VariantTag(cp_7377) = "Exist"
                -> LET (*
                  @type: (({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) => Bool);
                *)
                __QUINT_LAMBDA129(id__7372) == FALSE
                IN
                __QUINT_LAMBDA129(VariantGetUnsafe("Exist", cp_7377))
          IN
          __QUINT_LAMBDA130(VariantGetUnsafe("Some", (ics23_prove(tree_7387, k_7382,
          v_7387))))
        [] VariantTag((ics23_prove(tree_7387, k_7382, v_7387))) = "None"
          -> LET (*
            @type: (({ tag: Str }) => Bool);
          *)
          __QUINT_LAMBDA131(id__7380) == FALSE
          IN
          __QUINT_LAMBDA131(VariantGetUnsafe("None", (ics23_prove(tree_7387, k_7382,
          v_7387))))

(*
  @type: (() => Bool);
*)
verifyMembershipInv ==
  \A version_1118 \in versionsToCheck:
    LET (*
      @type: (() => Set({ key_hash: Seq(Int), value_hash: Seq(Int) }));
    *)
    leafs == allLeafs((treeAtVersion(tree, version_1118)))
    IN
    LET (*
      @type: (() => (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))));
    *)
    root ==
      hash(tree["nodes"][[key_hash |-> ROOT_BITS, version |-> version_1118]])
    IN
    has(tree["nodes"], [key_hash |-> ROOT_BITS, version |-> version_1118])
      => (\A key_hash_1113 \in all_key_hashes:
        LET (*
          @type: (() => None({ tag: Str }) | Some(Exist({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) | NonExist({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) })));
        *)
        proof == ics23_prove(tree, key_hash_1113, version_1118)
        IN
        CASE VariantTag((proof)) = "Some"
            -> LET (*
              @type: ((Exist({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) | NonExist({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) })) => Bool);
            *)
            __QUINT_LAMBDA146(p_1107) ==
              CASE VariantTag(p_1107) = "Exist"
                  -> LET (*
                    @type: (({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) => Bool);
                  *)
                  __QUINT_LAMBDA144(ep_1099) ==
                    (\E l_1061 \in leafs:
                        l_1061["key_hash"] = key_hash_1113
                          /\ verifyMembership((root), ep_1099, l_1061[
                            "key_hash"
                          ], l_1061["value_hash"])
                          /\ (\A value_hash_1058 \in all_value_hashes
                            \ {l_1061["value_hash"]}:
                            ~(verifyMembership((root), ep_1099, key_hash_1113, value_hash_1058))))
                      /\ (\A key_hash_1078 \in all_key_hashes \ {key_hash_1113}:
                        \A value_hash_1076 \in all_value_hashes:
                          ~(verifyMembership((root), ep_1099, key_hash_1078, value_hash_1076)))
                  IN
                  __QUINT_LAMBDA144(VariantGetUnsafe("Exist", p_1107))
                [] VariantTag(p_1107) = "NonExist"
                  -> LET (*
                    @type: (({ key: Seq(Int), left: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }), right: None({ tag: Str }) | Some({ key: Seq(Int), leaf: { prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }, path: Seq({ prefix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), suffix: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))) }), value: Seq(Int) }) }) => Bool);
                  *)
                  __QUINT_LAMBDA145(nep_1102) ==
                    verifyNonMembership((root), nep_1102, key_hash_1113)
                      /\ (\A l_1094 \in leafs:
                        ~(verifyNonMembership((root), nep_1102, l_1094[
                          "key_hash"
                        ])))
                  IN
                  __QUINT_LAMBDA145(VariantGetUnsafe("NonExist", p_1107))
            IN
            __QUINT_LAMBDA146(VariantGetUnsafe("Some", (proof)))
          [] VariantTag((proof)) = "None"
            -> LET (*
              @type: (({ tag: Str }) => Bool);
            *)
            __QUINT_LAMBDA147(id__1110) == TRUE
            IN
            __QUINT_LAMBDA147(VariantGetUnsafe("None", (proof))))

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, Int, Seq(Int), Set({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) })) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
*)
fancy_create_subtree(fancy_tree_6884, fancy_version_6884, fancy_bits_6884, fancy_batch_6884,
fancy_existing_leaf_6884) ==
  LET (*
    @type: (() => (<<Int, Seq(Int), Set({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), value_hash: Seq(Int) })>> -> { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) }));
  *)
  fancy_memo ==
    fancy_pre_compute_create_subtree(fancy_tree_6884, fancy_version_6884, fancy_bits_6884,
    fancy_batch_6884, fancy_existing_leaf_6884)
  IN
  LET (*
    @type: (() => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
  *)
  fancy_result ==
    (fancy_memo)[
      <<
        fancy_version_6884, fancy_bits_6884, fancy_batch_6884, fancy_existing_leaf_6884
      >>
    ]
  IN
  [outcome |-> (fancy_result)["outcome"],
    orphans_to_add |-> {},
    nodes_to_add |-> (fancy_result)["nodes_to_add"]]

(*
  @type: (() => Bool);
*)
nonMembershipCompletenessInv ==
  \A v_974 \in versionsToCheck: nonMembershipCompleteness(tree, v_974)

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, Int, Seq(Int), { key_hash: Seq(Int), value_hash: Seq(Int) }, Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
*)
fancy_apply_at_leaf(fancy_tree_6668, fancy_new_version_6668, fancy_bits_6668, fancy_leaf_node_6668,
fancy_batch_6668) ==
  LET (*
    @type: (() => <<Set({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })>>);
  *)
  fancy_batchAndOp ==
    fancy_prepare_batch_for_subtree(fancy_batch_6668, (fancy_Some(fancy_leaf_node_6668)))
  IN
  LET (*
    @type: (() => Set({ key_hash: Seq(Int), value_hash: Seq(Int) }));
  *)
  fancy_batch_6666 == (fancy_batchAndOp)[1]
  IN
  LET (*
    @type: (() => None({ tag: Str }) | Some({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }));
  *)
  fancy_operation == (fancy_batchAndOp)[2]
  IN
  IF fancy_batch_6666 = {}
  THEN LET (*
    @type: (() => Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
  *)
  fancy_outcome ==
    CASE VariantTag((fancy_operation)) = "Some"
        -> LET (*
          @type: (({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }) => Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
        *)
        __QUINT_LAMBDA207(fancy_op_6606) ==
          CASE VariantTag(fancy_op_6606["op"]) = "Insert"
              -> LET (*
                @type: ((Seq(Int)) => Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
              *)
              __QUINT_LAMBDA205(fancy_value_hash_6595) ==
                IF fancy_value_hash_6595 = fancy_leaf_node_6668["value_hash"]
                THEN fancy_Unchanged((fancy_Some((fancy_Leaf(fancy_leaf_node_6668)))))
                ELSE LET (*
                  @type: (() => { key_hash: Seq(Int), value_hash: Seq(Int) });
                *)
                fancy_updated_leaf_node ==
                  [
                    fancy_leaf_node_6668 EXCEPT
                      !["value_hash"] = fancy_value_hash_6595
                  ]
                IN
                fancy_Updated((fancy_Leaf((fancy_updated_leaf_node))))
              IN
              __QUINT_LAMBDA205(VariantGetUnsafe("Insert", fancy_op_6606["op"]))
            [] VariantTag(fancy_op_6606["op"]) = "Delete"
              -> LET (*
                @type: (({ tag: Str }) => Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
              *)
              __QUINT_LAMBDA206(fancy___6598) == fancy_Deleted
              IN
              __QUINT_LAMBDA206(VariantGetUnsafe("Delete", fancy_op_6606["op"]))
        IN
        __QUINT_LAMBDA207(VariantGetUnsafe("Some", (fancy_operation)))
      [] VariantTag((fancy_operation)) = "None"
        -> LET (*
          @type: (({ tag: Str }) => Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
        *)
        __QUINT_LAMBDA208(fancy___6609) ==
          fancy_Unchanged((fancy_Some((fancy_Leaf(fancy_leaf_node_6668)))))
        IN
        __QUINT_LAMBDA208(VariantGetUnsafe("None", (fancy_operation)))
  IN
  [outcome |-> fancy_outcome, orphans_to_add |-> {}, nodes_to_add |-> {}]
  ELSE CASE VariantTag((fancy_operation)) = "Some"
      -> LET (*
        @type: (({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
      *)
      __QUINT_LAMBDA211(fancy_op_6659) ==
        CASE VariantTag(fancy_op_6659["op"]) = "Insert"
            -> LET (*
              @type: ((Seq(Int)) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
            *)
            __QUINT_LAMBDA209(fancy_value_hash_6645) ==
              LET (*
                @type: (() => { key_hash: Seq(Int), value_hash: Seq(Int) });
              *)
              fancy_updated_leaf_node ==
                [
                  fancy_leaf_node_6668 EXCEPT
                    !["value_hash"] = fancy_value_hash_6645
                ]
              IN
              fancy_create_subtree(fancy_tree_6668, fancy_new_version_6668, fancy_bits_6668,
              (fancy_batch_6666), (fancy_Some((fancy_updated_leaf_node))))
            IN
            __QUINT_LAMBDA209(VariantGetUnsafe("Insert", fancy_op_6659["op"]))
          [] VariantTag(fancy_op_6659["op"]) = "Delete"
            -> LET (*
              @type: (({ tag: Str }) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
            *)
            __QUINT_LAMBDA210(fancy___6648) ==
              fancy_create_subtree(fancy_tree_6668, fancy_new_version_6668, fancy_bits_6668,
              (fancy_batch_6666), (fancy_None))
            IN
            __QUINT_LAMBDA210(VariantGetUnsafe("Delete", fancy_op_6659["op"]))
      IN
      __QUINT_LAMBDA211(VariantGetUnsafe("Some", (fancy_operation)))
    [] VariantTag((fancy_operation)) = "None"
      -> LET (*
        @type: (({ tag: Str }) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
      *)
      __QUINT_LAMBDA212(fancy___6662) ==
        fancy_create_subtree(fancy_tree_6668, fancy_new_version_6668, fancy_bits_6668,
        (fancy_batch_6666), (fancy_Some(fancy_leaf_node_6668)))
      IN
      __QUINT_LAMBDA212(VariantGetUnsafe("None", (fancy_operation)))

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, (<<Int, Int, Seq(Int), Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })>> -> { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) }), Int, Seq(Int), None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
*)
fancy_apply_at_child(fancy_tree_6539, fancy_memo_6539, fancy_new_version_6539, fancy_child_bits_6539,
fancy_child_6539, fancy_batch_6539) ==
  IF fancy_batch_6539 = {}
  THEN CASE VariantTag(fancy_child_6539) = "None"
      -> LET (*
        @type: (({ tag: Str }) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
      *)
      __QUINT_LAMBDA213(fancy___6503) ==
        [outcome |-> fancy_Unchanged((fancy_None)),
          orphans_to_add |-> {},
          nodes_to_add |-> {}]
      IN
      __QUINT_LAMBDA213(VariantGetUnsafe("None", fancy_child_6539))
    [] VariantTag(fancy_child_6539) = "Some"
      -> LET (*
        @type: (({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
      *)
      __QUINT_LAMBDA214(fancy_child_6506) ==
        LET (*
          @type: (() => Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }));
        *)
        fancy_child_node ==
          fancy_tree_6539["nodes"][
            [version |-> fancy_child_6506["version"],
              key_hash |-> fancy_child_bits_6539]
          ]
        IN
        [outcome |-> fancy_Unchanged((fancy_Some((fancy_child_node)))),
          orphans_to_add |-> {},
          nodes_to_add |-> {}]
      IN
      __QUINT_LAMBDA214(VariantGetUnsafe("Some", fancy_child_6539))
  ELSE CASE VariantTag(fancy_child_6539) = "None"
      -> LET (*
        @type: (({ tag: Str }) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
      *)
      __QUINT_LAMBDA215(fancy___6533) ==
        LET (*
          @type: (() => <<Set({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })>>);
        *)
        fancy_batchAndOp ==
          fancy_prepare_batch_for_subtree(fancy_batch_6539, (fancy_None))
        IN
        fancy_create_subtree(fancy_tree_6539, fancy_new_version_6539, fancy_child_bits_6539,
        (fancy_batchAndOp)[1], (fancy_None))
      IN
      __QUINT_LAMBDA215(VariantGetUnsafe("None", fancy_child_6539))
    [] VariantTag(fancy_child_6539) = "Some"
      -> LET (*
        @type: (({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
      *)
      __QUINT_LAMBDA216(fancy_child_6536) ==
        fancy_memo_6539[
          <<
            fancy_new_version_6539, fancy_child_6536["version"], fancy_child_bits_6539,
            fancy_batch_6539
          >>
        ]
      IN
      __QUINT_LAMBDA216(VariantGetUnsafe("Some", fancy_child_6539))

(*
  @type: (() => Bool);
*)
proofInvariants ==
  (IF membershipCompletenessInv
    THEN TRUE
    ELSE q_debug("membershipCompletenessInv", FALSE))
    /\ (IF nonMembershipCompletenessInv
    THEN TRUE
    ELSE q_debug("nonMembershipCompletenessInv", FALSE))
    /\ (IF membershipSoundnessInv
    THEN TRUE
    ELSE q_debug("membershipSoundnessInv", FALSE))
    /\ (IF nonMembershipSoundnessInv
    THEN TRUE
    ELSE q_debug("nonMembershipSoundnessInv", FALSE))
    /\ (IF verifyMembershipInv
    THEN TRUE
    ELSE q_debug("verifyMembershipInv", FALSE))

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, (<<Int, Int, Seq(Int), Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })>> -> { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) }), Int, Seq(Int), { left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }, Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
*)
fancy_apply_at_internal(fancy_tree_6449, fancy_memo_6449, fancy_new_version_6449,
fancy_bits_6449, fancy_internal_node_6449, fancy_batch_6449) ==
  LET (*
    @type: (() => <<Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }), Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })>>);
  *)
  fancy_partitioned_batch ==
    fancy_partition_batch(fancy_batch_6449, fancy_bits_6449)
  IN
  LET (*
    @type: (() => Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }));
  *)
  fancy_batch_for_left == (fancy_partitioned_batch)[1]
  IN
  LET (*
    @type: (() => Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }));
  *)
  fancy_batch_for_right == (fancy_partitioned_batch)[2]
  IN
  LET (*
    @type: (() => Seq(Int));
  *)
  fancy_left_bits == Append(fancy_bits_6449, 0)
  IN
  LET (*
    @type: (() => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
  *)
  fancy_left_result ==
    fancy_apply_at_child(fancy_tree_6449, fancy_memo_6449, fancy_new_version_6449,
    (fancy_left_bits), fancy_internal_node_6449["left_child"], (fancy_batch_for_left))
  IN
  LET (*
    @type: (() => Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
  *)
  fancy_left_outcome == (fancy_left_result)["outcome"]
  IN
  LET (*
    @type: (() => Seq(Int));
  *)
  fancy_right_bits == Append(fancy_bits_6449, 1)
  IN
  LET (*
    @type: (() => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
  *)
  fancy_right_result ==
    fancy_apply_at_child(fancy_tree_6449, fancy_memo_6449, fancy_new_version_6449,
    (fancy_right_bits), fancy_internal_node_6449["right_child"], (fancy_batch_for_right))
  IN
  LET (*
    @type: (() => Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
  *)
  fancy_right_outcome == (fancy_right_result)["outcome"]
  IN
  LET (*
    @type: (() => Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }));
  *)
  fancy_left_orphans ==
    IF fancy_is_updated_or_deleted((fancy_left_outcome))
      /\ fancy_internal_node_6449["left_child"] /= fancy_None
    THEN {[orphaned_since_version |-> fancy_new_version_6449,
      version |->
        (fancy_unwrap(fancy_internal_node_6449["left_child"]))["version"],
      key_hash |-> fancy_left_bits]}
    ELSE {}
  IN
  LET (*
    @type: (() => Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }));
  *)
  fancy_right_orphans ==
    IF fancy_is_updated_or_deleted((fancy_right_outcome))
      /\ fancy_internal_node_6449["right_child"] /= fancy_None
    THEN {[orphaned_since_version |-> fancy_new_version_6449,
      version |->
        (fancy_unwrap(fancy_internal_node_6449["right_child"]))["version"],
      key_hash |-> fancy_right_bits]}
    ELSE {}
  IN
  LET (*
    @type: (() => Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }));
  *)
  fancy_orphans_from_children ==
    (((fancy_left_result)["orphans_to_add"]
      \union (fancy_right_result)["orphans_to_add"])
      \union fancy_left_orphans)
      \union fancy_right_orphans
  IN
  LET (*
    @type: (() => Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>));
  *)
  fancy_nodes_from_children ==
    (fancy_left_result)["nodes_to_add"]
      \union (fancy_right_result)["nodes_to_add"]
  IN
  LET (*
    @type: (() => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
  *)
  fancy_default_result ==
    [outcome |-> fancy_Unchanged((fancy_None)),
      orphans_to_add |-> fancy_orphans_from_children,
      nodes_to_add |-> fancy_nodes_from_children]
  IN
  IF fancy_is_unchanged((fancy_left_outcome))
    /\ fancy_is_unchanged((fancy_right_outcome))
  THEN [
    (fancy_default_result) EXCEPT
      !["outcome"] =
        fancy_Unchanged((fancy_Some((fancy_Internal(fancy_internal_node_6449)))))
  ]
  ELSE IF (fancy_left_outcome = fancy_Deleted
      \/ fancy_left_outcome = fancy_Unchanged((fancy_None)))
    /\ (fancy_right_outcome = fancy_Deleted
      \/ fancy_right_outcome = fancy_Unchanged((fancy_None)))
  THEN [ (fancy_default_result) EXCEPT !["outcome"] = fancy_Deleted ]
  ELSE IF fancy_updated_to_leaf((fancy_left_outcome))
    /\ (fancy_right_outcome = fancy_Deleted
      \/ fancy_right_outcome = fancy_Unchanged((fancy_None)))
  THEN [ (fancy_default_result) EXCEPT !["outcome"] = fancy_left_outcome ]
  ELSE IF fancy_unchanged_leaf((fancy_left_outcome))
    /\ fancy_right_outcome = fancy_Deleted
  THEN CASE VariantTag((fancy_left_outcome)) = "Unchanged"
      -> LET (*
        @type: ((None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
      *)
      __QUINT_LAMBDA232(fancy_left_6250) ==
        LET (*
          @type: (() => Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }));
        *)
        fancy_orphans ==
          (fancy_default_result)["orphans_to_add"]
            \union {[orphaned_since_version |-> fancy_new_version_6449,
              version |->
                (fancy_unwrap(fancy_internal_node_6449["left_child"]))[
                  "version"
                ],
              key_hash |-> fancy_left_bits]}
        IN
        [
          [
            (fancy_default_result) EXCEPT
              !["outcome"] = fancy_Updated((fancy_unwrap(fancy_left_6250)))
          ] EXCEPT
            !["orphans_to_add"] = fancy_orphans
        ]
      IN
      __QUINT_LAMBDA232(VariantGetUnsafe("Unchanged", (fancy_left_outcome)))
    [] OTHER
      -> LET (*
        @type: ((a78) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
      *)
      __QUINT_LAMBDA233(fancy___6253) == fancy_default_result
      IN
      __QUINT_LAMBDA233({})
  ELSE IF (fancy_left_outcome = fancy_Deleted
      \/ fancy_left_outcome = fancy_Unchanged((fancy_None)))
    /\ fancy_updated_to_leaf((fancy_right_outcome))
  THEN [ (fancy_default_result) EXCEPT !["outcome"] = fancy_right_outcome ]
  ELSE IF fancy_left_outcome = fancy_Deleted
    /\ fancy_unchanged_leaf((fancy_right_outcome))
  THEN CASE VariantTag((fancy_right_outcome)) = "Unchanged"
      -> LET (*
        @type: ((None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
      *)
      __QUINT_LAMBDA234(fancy_right_6308) ==
        LET (*
          @type: (() => Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }));
        *)
        fancy_orphans ==
          (fancy_default_result)["orphans_to_add"]
            \union {[orphaned_since_version |-> fancy_new_version_6449,
              version |->
                (fancy_unwrap(fancy_internal_node_6449["right_child"]))[
                  "version"
                ],
              key_hash |-> fancy_right_bits]}
        IN
        [
          [
            (fancy_default_result) EXCEPT
              !["outcome"] = fancy_Updated((fancy_unwrap(fancy_right_6308)))
          ] EXCEPT
            !["orphans_to_add"] = fancy_orphans
        ]
      IN
      __QUINT_LAMBDA234(VariantGetUnsafe("Unchanged", (fancy_right_outcome)))
    [] OTHER
      -> LET (*
        @type: ((a79) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
      *)
      __QUINT_LAMBDA235(fancy___6311) == fancy_default_result
      IN
      __QUINT_LAMBDA235({})
  ELSE LET (*
    @type: (() => { child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>) });
  *)
  fancy_new_left_child_and_tree ==
    CASE VariantTag((fancy_left_outcome)) = "Updated"
        -> LET (*
          @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => { child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>) });
        *)
        __QUINT_LAMBDA236(fancy_node_6346) ==
          [child |->
              fancy_Some([version |-> fancy_new_version_6449,
                hash |-> fancy_hash(fancy_node_6346)]),
            nodes_to_add |->
              {<<
                [version |-> fancy_new_version_6449,
                  key_hash |-> fancy_left_bits], fancy_node_6346
              >>}]
        IN
        __QUINT_LAMBDA236(VariantGetUnsafe("Updated", (fancy_left_outcome)))
      [] VariantTag((fancy_left_outcome)) = "Deleted"
        -> LET (*
          @type: (({ tag: Str }) => { child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>) });
        *)
        __QUINT_LAMBDA237(fancy___6349) ==
          [child |-> fancy_None, nodes_to_add |-> {}]
        IN
        __QUINT_LAMBDA237(VariantGetUnsafe("Deleted", (fancy_left_outcome)))
      [] VariantTag((fancy_left_outcome)) = "Unchanged"
        -> LET (*
          @type: ((None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => { child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>) });
        *)
        __QUINT_LAMBDA238(fancy___6352) ==
          [child |-> fancy_internal_node_6449["left_child"],
            nodes_to_add |-> {}]
        IN
        __QUINT_LAMBDA238(VariantGetUnsafe("Unchanged", (fancy_left_outcome)))
  IN
  LET (*
    @type: (() => { child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>) });
  *)
  fancy_new_right_child_and_tree ==
    CASE VariantTag((fancy_right_outcome)) = "Updated"
        -> LET (*
          @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => { child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>) });
        *)
        __QUINT_LAMBDA239(fancy_node_6388) ==
          [child |->
              fancy_Some([version |-> fancy_new_version_6449,
                hash |-> fancy_hash(fancy_node_6388)]),
            nodes_to_add |->
              {<<
                [version |-> fancy_new_version_6449,
                  key_hash |-> fancy_right_bits], fancy_node_6388
              >>}]
        IN
        __QUINT_LAMBDA239(VariantGetUnsafe("Updated", (fancy_right_outcome)))
      [] VariantTag((fancy_right_outcome)) = "Deleted"
        -> LET (*
          @type: (({ tag: Str }) => { child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>) });
        *)
        __QUINT_LAMBDA240(fancy___6391) ==
          [child |-> fancy_None, nodes_to_add |-> {}]
        IN
        __QUINT_LAMBDA240(VariantGetUnsafe("Deleted", (fancy_right_outcome)))
      [] VariantTag((fancy_right_outcome)) = "Unchanged"
        -> LET (*
          @type: ((None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => { child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>) });
        *)
        __QUINT_LAMBDA241(fancy___6394) ==
          [child |-> fancy_internal_node_6449["right_child"],
            nodes_to_add |-> {}]
        IN
        __QUINT_LAMBDA241(VariantGetUnsafe("Unchanged", (fancy_right_outcome)))
  IN
  LET (*
    @type: (() => Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }));
  *)
  fancy_new_internal_node ==
    fancy_Internal([left_child |-> (fancy_new_left_child_and_tree)["child"],
      right_child |-> (fancy_new_right_child_and_tree)["child"]])
  IN
  [
    [
      (fancy_default_result) EXCEPT
        !["outcome"] = fancy_Updated((fancy_new_internal_node))
    ] EXCEPT
      !["nodes_to_add"] =
        ((fancy_default_result)["nodes_to_add"]
          \union (fancy_new_left_child_and_tree)["nodes_to_add"])
          \union (fancy_new_right_child_and_tree)["nodes_to_add"]
  ]

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, (<<Int, Int, Seq(Int), Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })>> -> { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) }), Int, Int, Seq(Int), Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
*)
fancy_apply_at(fancy_tree_6023, fancy_memo_6023, fancy_new_version_6023, fancy_old_version_6023,
fancy_bits_6023, fancy_batch_6023) ==
  CASE VariantTag((fancy_safeGet(fancy_tree_6023["nodes"], [version |->
        fancy_old_version_6023,
      key_hash |-> fancy_bits_6023])))
      = "Some"
      -> LET (*
        @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
      *)
      __QUINT_LAMBDA244(fancy_node_6018) ==
        CASE VariantTag(fancy_node_6018) = "Leaf"
            -> LET (*
              @type: (({ key_hash: Seq(Int), value_hash: Seq(Int) }) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
            *)
            __QUINT_LAMBDA242(fancy_leaf_node_5998) ==
              fancy_apply_at_leaf(fancy_tree_6023, fancy_new_version_6023, fancy_bits_6023,
              fancy_leaf_node_5998, fancy_batch_6023)
            IN
            __QUINT_LAMBDA242(VariantGetUnsafe("Leaf", fancy_node_6018))
          [] VariantTag(fancy_node_6018) = "Internal"
            -> LET (*
              @type: (({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
            *)
            __QUINT_LAMBDA243(fancy_internal_node_6001) ==
              fancy_apply_at_internal(fancy_tree_6023, fancy_memo_6023, fancy_new_version_6023,
              fancy_bits_6023, fancy_internal_node_6001, fancy_batch_6023)
            IN
            __QUINT_LAMBDA243(VariantGetUnsafe("Internal", fancy_node_6018))
      IN
      __QUINT_LAMBDA244(VariantGetUnsafe("Some", (fancy_safeGet(fancy_tree_6023[
        "nodes"
      ], [version |-> fancy_old_version_6023, key_hash |-> fancy_bits_6023]))))
    [] VariantTag((fancy_safeGet(fancy_tree_6023["nodes"], [version |->
        fancy_old_version_6023,
      key_hash |-> fancy_bits_6023])))
      = "None"
      -> LET (*
        @type: (({ tag: Str }) => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
      *)
      __QUINT_LAMBDA245(fancy___6021) ==
        LET (*
          @type: (() => <<Set({ key_hash: Seq(Int), value_hash: Seq(Int) }), None({ tag: Str }) | Some({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })>>);
        *)
        fancy_batchAndOp ==
          fancy_prepare_batch_for_subtree(fancy_batch_6023, (fancy_None))
        IN
        fancy_create_subtree(fancy_tree_6023, fancy_new_version_6023, fancy_bits_6023,
        (fancy_batchAndOp)[1], (fancy_None))
      IN
      __QUINT_LAMBDA245(VariantGetUnsafe("None", (fancy_safeGet(fancy_tree_6023[
        "nodes"
      ], [version |-> fancy_old_version_6023, key_hash |-> fancy_bits_6023]))))

(*
  @type: (() => Bool);
*)
allInvariants == treeInvariants /\ proofInvariants

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, Int, Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })) => (<<Int, Int, Seq(Int), Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })>> -> { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) }));
*)
fancy_pre_compute_apply_at(fancy_tree_5844, fancy_new_version_5844, fancy_batch_5844) ==
  LET (*
    @type: (((<<Int, Int, Seq(Int), Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })>> -> { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) }), Int) => (<<Int, Int, Seq(Int), Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })>> -> { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) }));
  *)
  __QUINT_LAMBDA248(fancy_memo_5842, fancy_old_version_5842) ==
    LET (*
      @type: (({ key_hash: Seq(Int), version: Int }, (<<Int, Int, Seq(Int), Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })>> -> { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) })) => (<<Int, Int, Seq(Int), Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })>> -> { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) }));
    *)
    __QUINT_LAMBDA247(fancy_node_id_5840, fancy_memo_5840) ==
      LET (*
        @type: (() => Seq(Int));
      *)
      fancy_bits == fancy_node_id_5840["key_hash"]
      IN
      LET (*
        @type: (() => Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }));
      *)
      fancy_batch_here ==
        {
          fancy_o_5815 \in fancy_batch_5844:
            fancy_prefix_of((fancy_bits), fancy_o_5815["key_hash"])
        }
      IN
      LET (*
        @type: (() => <<Int, Int, Seq(Int), Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })>>);
      *)
      fancy_memo_key ==
        <<
          fancy_new_version_5844, fancy_old_version_5842, (fancy_bits), (fancy_batch_here)
        >>
      IN
      LET (*
        @type: (() => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
      *)
      fancy_memo_value ==
        fancy_apply_at(fancy_tree_5844, fancy_memo_5840, fancy_new_version_5844,
        fancy_old_version_5842, (fancy_bits), (fancy_batch_here))
      IN
      LET (*
        @type: (() => (<<Int, Int, Seq(Int), Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })>> -> { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) }));
      *)
      __quint_var40 == fancy_memo_5840
      IN
      LET (*
        @type: (() => Set(<<Int, Int, Seq(Int), Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })>>));
      *)
      __quint_var41 == DOMAIN __quint_var40
      IN
      [
        __quint_var42 \in {(fancy_memo_key)} \union __quint_var41 |->
          IF __quint_var42 = fancy_memo_key
          THEN fancy_memo_value
          ELSE (__quint_var40)[__quint_var42]
      ]
    IN
    foldr((fancy_sorted_nodes(fancy_tree_5844)), fancy_memo_5842, __QUINT_LAMBDA247)
  IN
  ApaFoldSeqLeft(__QUINT_LAMBDA248, (SetAsFun({})), (LET (*
    @type: ((Int) => Int);
  *)
  __QUINT_LAMBDA246(__quint_var39) == (0 + __quint_var39) - 1
  IN
  MkSeq((fancy_new_version_5844 - 0), __QUINT_LAMBDA246)))

(*
  @type: (() => Bool);
*)
q_inv == allInvariants

(*
  @type: (({ nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) }, Int, Int, Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })) => { nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) });
*)
fancy_apply(fancy_tree_5957, fancy_old_version_5957, fancy_new_version_5957, fancy_batch_5957) ==
  LET (*
    @type: (() => { nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) });
  *)
  fancy_tree_1 ==
    IF fancy_has(fancy_tree_5957["nodes"], [version |-> fancy_old_version_5957,
      key_hash |-> fancy_ROOT_BITS])
    THEN fancy_mark_node_as_orphaned(fancy_tree_5957, fancy_new_version_5957, fancy_old_version_5957,
    (fancy_ROOT_BITS))
    ELSE fancy_tree_5957
  IN
  LET (*
    @type: (() => (<<Int, Int, Seq(Int), Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) })>> -> { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) }));
  *)
  fancy_memo ==
    fancy_pre_compute_apply_at((fancy_tree_1), fancy_new_version_5957, fancy_batch_5957)
  IN
  LET (*
    @type: (() => { nodes_to_add: Set(<<{ key_hash: Seq(Int), version: Int }, Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })>>), orphans_to_add: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }), outcome: Deleted({ tag: Str }) | Unchanged(None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) | Updated(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) });
  *)
  fancy_apply_result ==
    fancy_apply_at((fancy_tree_1), (fancy_memo), fancy_new_version_5957, fancy_old_version_5957,
    (fancy_ROOT_BITS), fancy_batch_5957)
  IN
  LET (*
    @type: (() => { nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) });
  *)
  fancy_new_tree ==
    [nodes |->
        fancy_add_nodes((fancy_tree_1)["nodes"], (fancy_apply_result)[
          "nodes_to_add"
        ]),
      orphans |->
        (fancy_tree_1)["orphans"] \union (fancy_apply_result)["orphans_to_add"]]
  IN
  CASE VariantTag((fancy_apply_result)["outcome"]) = "Updated"
      -> LET (*
        @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => { nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) });
      *)
      __QUINT_LAMBDA250(fancy_new_root_node_5945) ==
        [
          (fancy_new_tree) EXCEPT
            !["nodes"] =
              LET (*
                @type: (() => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
              *)
              __quint_var46 == (fancy_new_tree)["nodes"]
              IN
              LET (*
                @type: (() => Set({ key_hash: Seq(Int), version: Int }));
              *)
              __quint_var47 == DOMAIN __quint_var46
              IN
              [
                __quint_var48 \in
                  {[version |-> fancy_new_version_5957,
                    key_hash |-> fancy_ROOT_BITS]}
                    \union __quint_var47 |->
                  IF __quint_var48
                    = [version |-> fancy_new_version_5957,
                      key_hash |-> fancy_ROOT_BITS]
                  THEN fancy_new_root_node_5945
                  ELSE (__quint_var46)[__quint_var48]
              ]
        ]
      IN
      __QUINT_LAMBDA250(VariantGetUnsafe("Updated", (fancy_apply_result)[
        "outcome"
      ]))
    [] VariantTag((fancy_apply_result)["outcome"]) = "Unchanged"
      -> LET (*
        @type: ((None({ tag: Str }) | Some(Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) }))) => { nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) });
      *)
      __QUINT_LAMBDA253(fancy_optional_5948) ==
        CASE VariantTag(fancy_optional_5948) = "Some"
            -> LET (*
              @type: ((Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })) => { nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) });
            *)
            __QUINT_LAMBDA251(fancy_new_root_node_5937) ==
              [
                (fancy_new_tree) EXCEPT
                  !["nodes"] =
                    LET (*
                      @type: (() => ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })));
                    *)
                    __quint_var49 == (fancy_new_tree)["nodes"]
                    IN
                    LET (*
                      @type: (() => Set({ key_hash: Seq(Int), version: Int }));
                    *)
                    __quint_var50 == DOMAIN __quint_var49
                    IN
                    [
                      __quint_var51 \in
                        {[version |-> fancy_new_version_5957,
                          key_hash |-> fancy_ROOT_BITS]}
                          \union __quint_var50 |->
                        IF __quint_var51
                          = [version |-> fancy_new_version_5957,
                            key_hash |-> fancy_ROOT_BITS]
                        THEN fancy_new_root_node_5937
                        ELSE (__quint_var49)[__quint_var51]
                    ]
              ]
            IN
            __QUINT_LAMBDA251(VariantGetUnsafe("Some", fancy_optional_5948))
          [] VariantTag(fancy_optional_5948) = "None"
            -> LET (*
              @type: (({ tag: Str }) => { nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) });
            *)
            __QUINT_LAMBDA252(fancy___5940) == fancy_new_tree
            IN
            __QUINT_LAMBDA252(VariantGetUnsafe("None", fancy_optional_5948))
      IN
      __QUINT_LAMBDA253(VariantGetUnsafe("Unchanged", (fancy_apply_result)[
        "outcome"
      ]))
    [] OTHER
      -> LET (*
        @type: ((a82) => { nodes: ({ key_hash: Seq(Int), version: Int } -> Internal({ left_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }), right_child: None({ tag: Str }) | Some({ hash: (Seq(Int) -> Hash({ tag: Str }) | Raw(Seq(Int))), version: Int }) }) | Leaf({ key_hash: Seq(Int), value_hash: Seq(Int) })), orphans: Set({ key_hash: Seq(Int), orphaned_since_version: Int, version: Int }) });
      *)
      __QUINT_LAMBDA254(fancy___5951) == fancy_new_tree
      IN
      __QUINT_LAMBDA254({})

(*
  @type: (() => Bool);
*)
init ==
  \E kms_with_value \in [(all_key_hashes) -> (INIT_VALUES)]:
    LET (*
      @type: (() => Set({ key_hash: Seq(Int), op: Delete({ tag: Str }) | Insert(Seq(Int)) }));
    *)
    ops == to_operations(kms_with_value)
    IN
    tree
        = (fancy_apply([nodes |-> SetAsFun({}), orphans |-> {}], 0, 1, (ops)))
      /\ version = 2
      /\ smallest_unpruned_version = 1
      /\ ops_history = <<(ops)>>

(*
  @type: (() => Bool);
*)
step_fancy == step_parametrized(fancy_apply, assign_result)

(*
  @type: (() => Bool);
*)
q_init == init

(*
  @type: (() => Bool);
*)
q_step == step_fancy

================================================================================
