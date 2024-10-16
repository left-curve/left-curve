module helpers{
    import rareSpells.* from "./spells/rareSpells"
    
    pure def tupleCompare(__a: (int, List[int]), __b: (int, List[int])): Ordering = {
        if (__a._1 < __b._1)
        { LT }
        else if (__a._1 > __b._1)
        { GT }
        else
        { EQ }
    }
}