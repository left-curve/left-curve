import diatypeRegular from "@left-curve/foundation/fonts/ABCDiatypeRounded/files/ABCDiatypeRounded-Regular.woff2?inline";
import diatypeMedium from "@left-curve/foundation/fonts/ABCDiatypeRounded/files/ABCDiatypeRounded-Medium.woff2?inline";
import diatypeHeavy from "@left-curve/foundation/fonts/ABCDiatypeRounded/files/ABCDiatypeRounded-Heavy.woff2?inline";
import exposureItalic from "@left-curve/foundation/fonts/Exposure/files/Exposure-30-Italic-205TF.woff2?inline";

export const shareCardFontEmbedCSS = `
@font-face {
  font-family: 'ABCDiatypeRounded';
  font-style: normal;
  font-weight: 400;
  font-display: block;
  src: url(${diatypeRegular}) format('woff2');
}
@font-face {
  font-family: 'ABCDiatypeRounded';
  font-style: normal;
  font-weight: 500;
  font-display: block;
  src: url(${diatypeMedium}) format('woff2');
}
@font-face {
  font-family: 'ABCDiatypeRounded';
  font-style: normal;
  font-weight: 700;
  font-display: block;
  src: url(${diatypeHeavy}) format('woff2');
}
@font-face {
  font-family: 'Exposure';
  font-style: italic;
  font-weight: 700;
  font-display: block;
  src: url(${exposureItalic}) format('woff2');
}
`;
