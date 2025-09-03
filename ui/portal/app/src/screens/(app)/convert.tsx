import { Applet } from "~/components/foundation/Applet";

export default function ConvertApplet() {
  return <Applet uri={__DEV__ ? "http://localhost:5180/" : "https://convert.dango.exchange"} />;
}
