import { Footer } from "../components/Footer";
import { Fullpage } from "../components/Fullpage";
import { MottoSection } from "../components/MottoSection";

export default function Home() {
  return (
    <>
      <div className="fixed mx-0 top-6 z-50">
        <img src="/images/dango.svg" alt="logo" className="h-10 md:h-16 object-contain" />
      </div>
      <div className="w-screen h-screen absolute">
        <Fullpage>
          <MottoSection />
          <Footer />
        </Fullpage>
      </div>
    </>
  );
}
