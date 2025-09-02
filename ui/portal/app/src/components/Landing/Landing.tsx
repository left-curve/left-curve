import { Dimensions, Image, ScrollView, type ScrollViewProps, View } from "react-native";
import { GlobalText, FullSection } from "../foundation";

import { cssInterop } from "nativewind";

import Logo from "~/assets/images/dango.svg";
import { AppletsSection } from "./AppletSection";

const { height } = Dimensions.get("window");

cssInterop(Logo, {
  className: {
    target: "style",
    nativeStyleToProp: {
      fill: true,
      color: true,
      stroke: true,
      width: true,
      height: true,
    },
  },
});

const LandingContainer: React.FC<React.PropsWithChildren<ScrollViewProps>> = ({
  children,
  ...rest
}) => {
  return (
    <View
      className="flex-1 w-full"
      contentContainerClassName="mx-auto flex flex-col gap-6 h-screen"
      showsVerticalScrollIndicator={false}
      {...rest}
    >
      {children}
    </View>
  );
};

const Header: React.FC = () => {
  return (
    <View
      className="flex items-center justify-between w-full flex-1"
      style={{ height: height - 100 }}
    >
      <View className="w-full items-center">
        <Logo width={200} height={80} className="text-primary-900" />
      </View>

      <View className="w-full pb-[4rem]">
        <AppletsSection />
      </View>
    </View>
  );
};

const SectionRice: React.FC = () => {
  return (
    <FullSection
      lightGradient={["rgba(255,229,190,0.4)", "#FFDEAE"]}
      darkGradient={["#42403D", "#807668"]}
    >
      <View className="w-full max-w-[1216px] self-center flex-col lg:flex-row items-center lg:justify-between">
        <View className="max-w-[640px] self-center">
          <GlobalText className="display-heading-l text-[#9C4D21] dark:text-foreground-primary-rice">
            Trade
          </GlobalText>
          <GlobalText className="display-heading-l text-[#9C4D21] dark:text-foreground-primary-rice">
            crypto assets, real world assets, and derivatives on Dango’s blazingly fast exchange.
          </GlobalText>
          <GlobalText className="display-heading-l text-[#9C4D21] dark:text-foreground-primary-rice">
            Enjoy deep liquidity, fast execution, and fair prices.
          </GlobalText>
        </View>

        {/*  <Image source={{ uri: "" }} resizeMode="contain" style={{ width: 320, height: 320 }} /> */}
      </View>
    </FullSection>
  );
};

const SectionRed: React.FC = () => {
  return (
    <FullSection
      lightGradient={["rgba(255,221,223,0.4)", "#FFD0D3"]}
      darkGradient={["#383634", "#6A6361"]}
    >
      <View className="w-full max-w-[1216px] self-center px-0">
        <View className="max-w-[640px] self-start">
          <GlobalText className="display-heading-l text-tertiary-red">Leverage up</GlobalText>
          <GlobalText className="diatype-m-regular mt-2 text-tertiary-red">
            with Dango’s unified trading account with low cost and high capital efficiency.
          </GlobalText>
          <GlobalText className="diatype-m-regular mt-2 text-tertiary-red">
            Spot, perps, vaults; one account, under a unified margin system.
          </GlobalText>
        </View>

        <Image source={{ uri: "" }} resizeMode="contain" style={{ width: 320, height: 320 }} />
      </View>
    </FullSection>
  );
};

const SectionGreen: React.FC = () => {
  return (
    <FullSection
      lightGradient={["rgba(239,240,173,0.4)", "#EFF0AD"]}
      darkGradient={["#373634", "#666654"]}
    >
      <View className="w-full max-w-[1216px] self-center flex-col lg:flex-row items-center lg:justify-between">
        <View className="max-w-[640px] self-center">
          <GlobalText className="display-heading-l text-green-bean-800 dark:text-foreground-primary-green">
            Earn
          </GlobalText>
          <GlobalText className="diatype-m-regular mt-2 text-green-bean-800 dark:text-foreground-primary-green">
            passive yields on your idle assets, by participating in Dango’s passive market making
            vaults.
          </GlobalText>
        </View>

        <Image source={{ uri: "" }} resizeMode="contain" style={{ width: 320, height: 320 }} />
      </View>
    </FullSection>
  );
};

export const Landing = Object.assign(LandingContainer, {
  Header,
  SectionRice,
  SectionRed,
  SectionGreen,
});
