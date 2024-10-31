import ReactFullpage from "@fullpage/react-fullpage";
import type { PropsWithChildren } from "react";
import { Hero } from "./Hero";

interface Props {
  hero: React.FC<any>;
}

export const Fullpage: React.FC<PropsWithChildren<Props>> = ({ children }) => {
  return (
    <ReactFullpage
      licenseKey={"KEY"}
      scrollingSpeed={1000}
      credits={{ enabled: false }}
      render={({ fullpageApi }) => {
        return (
          <ReactFullpage.Wrapper>
            <div className="header top-0 w-screen h-[115vh] absolute" />
            <Hero goSectionBelow={() => fullpageApi.moveSectionDown()} />

            <div className="w-screen section">
              <div className="h-[100svh] flex flex-col items-center justify-center px-4 md:pt-24">
                {children}
              </div>
            </div>
          </ReactFullpage.Wrapper>
        );
      }}
    />
  );
};
