interface Props {
  goSectionBelow: () => void;
}

export const Hero: React.FC<Props> = ({ goSectionBelow }) => {
  return (
    <div className="section justify-center items-center h-screen w-screen overflow-x-hidden">
      <div className="w-screen h-screen flex flex-col items-center justify-evenly px-2 z-10 flex-1 relative pt-24 md:pb-4">
        <div className="h-[60%] md:h-[80%] 3xl:h-[90%] w-full relative max-w-[1240px] 3xl:max-w-[1440px] z-20 mx-auto">
          <div className="scale w-[36.11%] h-[38.54%] md:w-[26.25%] md:h-[45.54%] absolute left-[58.09%] top-[6.16%] sm:top-[1.16%] md:left-[44.31%] md:top-[2.15%] transition-all z-[0]">
            <div className="w-full h-full transition-all hover:scale-110">
              <img
                src="/images/rabbits.svg"
                alt="rabbits"
                className="object-fit w-full h-full transition-all rabbit"
              />
            </div>
          </div>
          <div className="scale w-[35.07%] h-[53.47%] left-[64.93%] top-[30.95%] sm:top-[27.95%] md:w-[27.08%] md:h-[67.12%] object-fit absolute md:left-[72.92%] md:top-[9.94%] hover:scale-[1.1] transition-all z-[3] md:z-[2] delay-3">
            <div className="w-full h-full transition-all hover:scale-110">
              <img
                src="/images/monkeys.svg"
                alt="monkeys"
                className="object-fit w-full h-full transition-all monkey"
              />
            </div>
          </div>
          <div className="scale w-[69.03%] h-[67.12%] object-fit absolute left-0 md:left-[23.40%] top-[33%] sm:top-[28.88%] md:top-[32.88%] hover:scale-[1.1] transition-all z-[4] hover:z-[6] delay-4">
            <div className="scale-x-[-1] md:scale-x-100 w-full h-full hover:scale-x-[-1.1] hover:scale-y-110 transition-all md:hover:scale-110">
              <img
                src="/images/dancer.svg"
                alt="dancer"
                className="object-fit w-full h-full transition-all dancer"
              />
            </div>
          </div>
          <div className="scale w-[63.89%] h-[52.78%] md:w-[51.11%] md:h-[68.81%] object-fit absolute left-0 top-0 md:left-[2.08%] md:top-[3.39%] transition-all z-[1] delay-2">
            <div className="w-full h-full transition-all hover:scale-110">
              <img
                src="/images/kiosko.svg"
                alt="kisko"
                className="object-fit w-full h-full transition-all kiosko"
              />
            </div>
          </div>
          <div className="scale w-[23.68%] h-[31.29%] object-fit absolute left-[13.13%] top-[47.34%] hover:scale-[1.1] transition-all z-[5] delay-5 hidden md:block">
            <div className="w-full h-full transition-all hover:scale-110">
              <img
                src="/images/dog.svg"
                alt="dog"
                className="object-fit w-full h-full transition-all dog"
              />
            </div>
          </div>
          <div className="scale w-[30.21%] h-[29.51%] md:w-[24.44%] md:h-[38.98%] object-fit absolute left-[37.59%] top-[37%] sm:top-[28.96%] md:left-[0%] md:top-[57.97%] hover:scale-[1.1] transition-all z-[2] md:z-[7] delay-6">
            <div className="w-full h-full transition-all hover:scale-110">
              <img
                src="/images/snake.svg"
                alt="snake"
                className="object-fit w-full h-full transition-all snake"
              />
            </div>
          </div>
        </div>
        <button
          type="button"
          onClick={goSectionBelow}
          className="text-typography-purple-300 hover:text-typography-purple-400 cursor-pointer"
        >
          <svg
            width="32"
            height="32"
            viewBox="0 0 32 32"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M2.85712 9.42859C5.80283 15.32 8.74856 18.5706 14.0619 21.5478C15.2649 22.2219 16.7351 22.2219 17.938 21.5478C23.2513 18.5706 26.1971 15.32 29.1428 9.42859"
              stroke="currentColor"
              strokeWidth="2.66667"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </button>
      </div>
    </div>
  );
};
