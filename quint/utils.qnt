module utils {
  import basicSpells.* from "./spells/basicSpells"
  pure def unwrap(value: Option[a]): a = {
    match value {
      | None => Map().get(value)
      | Some(x) => x
    }
  }
  pure def getOnlyElement(s: Set[a]): a = {
    pure val hack = s.map(e => ("value", e)).setToMap()
    if (s.size() != 1) {
      hack.get("error: expected singleton")
    } else {
      hack.get("value")
    }
  }

}