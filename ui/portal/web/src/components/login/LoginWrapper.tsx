import { Birdo, Carousel, ResizerContainer } from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store-react";
import { useNavigate } from "@tanstack/react-router";
import type React from "react";
import { useEffect } from "react";

export const LoginWrapper: React.FC<React.PropsWithChildren> = ({ children }) => {
  const { isConnected } = useAccount();
  const navigate = useNavigate();

  useEffect(() => {
    if (isConnected) navigate({ to: "/" });
  }, []);

  return (
    <div className="h-screen w-screen flex items-center justify-center">
      <div className="flex items-center justify-center flex-1">
        <ResizerContainer className="w-full max-w-[22.5rem]">{children}</ResizerContainer>
      </div>
      <div className="custom-width h-full min-w-[720px] w-[720px] hidden xl:flex bg-[url('./images/frame-rounded.svg')] bg-no-repeat bg-cover bg-center items-center justify-center">
        <Carousel>
          <div className="flex flex-col items-center justify-center gap-12">
            <Birdo className="max-w-[28.125rem] h-auto" />
            <div className="flex flex-col items-center justify-center gap-1 max-w-[25rem]">
              <h3 className="exposure-h3-italic">Welcome home</h3>
              <p className="text-gray-500 text-md">The good old days are here to stay.</p>
            </div>
          </div>
          <div className="flex flex-col items-center justify-center gap-12">
            <Birdo className="max-w-[28.125rem] h-auto" />
            <div className="flex flex-col items-center justify-center gap-1 max-w-[25rem]">
              <h3 className="exposure-h3-italic">Use Dango</h3>
              <p className="text-gray-500 text-md">
                Lorem ipsum dolor sit amet, consectetur adipiscing elit.
              </p>
            </div>
          </div>
          <div className="flex flex-col items-center justify-center gap-12">
            <Birdo className="max-w-[28.125rem] h-auto" />
            <div className="flex flex-col items-center justify-center gap-1 max-w-[25rem]">
              <h3 className="exposure-h3-italic">How to use it</h3>
              <p className="text-gray-500 text-md">Fusce purus justo, lobortis aliquet orci.</p>
            </div>
          </div>
        </Carousel>
      </div>
    </div>
  );
};
