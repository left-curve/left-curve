import { Applet } from "~/components/foundation/Applet";

export default function ConverApplet() {
  return <Applet uri={__DEV__ ? "http://localhost:5180/" : ""} />;
}
