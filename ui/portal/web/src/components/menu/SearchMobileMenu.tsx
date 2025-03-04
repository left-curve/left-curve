import { IconButton, IconChevronLeft, IconSearch, Input } from "@left-curve/applets-kit";
import type React from "react";
import { Sheet } from "react-modal-sheet";
import { SearchMenuBody } from "./SearchBody";

interface Props {
  isVisible: boolean;
  setVisibility: (isVisible: boolean) => void;
}

export const SearchMobileMenu: React.FC<Props> = ({ isVisible, setVisibility }) => {
  return (
    <Sheet isOpen={isVisible} onClose={() => setVisibility(false)} initialSnap={0}>
      <Sheet.Container className="!bg-white !rounded-t-2xl !shadow-none">
        <Sheet.Header />
        <Sheet.Content>
          <div className="flex flex-col gap-4 px-4">
            <div className="flex gap-2  items-center justify-center">
              <IconButton size="xs" variant="link" onClick={() => setVisibility(false)}>
                <IconChevronLeft />
              </IconButton>
              <Input
                fullWidth
                classNames={{
                  inputWrapper: "px-3",
                }}
                startContent={<IconSearch className="w-5 h-5 text-gray-500" />}
                placeholder="Search for apps"
              />
            </div>
            <SearchMenuBody />
          </div>
        </Sheet.Content>
      </Sheet.Container>
      <Sheet.Backdrop onTap={() => setVisibility(false)} />
    </Sheet>
  );
};
