import { useForm } from "react-hook-form";
import { useAccountName } from "../../hooks";
import { Button } from "../atoms/Button";
import { Input } from "../atoms/Input";

interface Props {
  goBack: () => void;
}

export const SpotEditAccount: React.FC<Props> = ({ goBack }) => {
  const [accoutName, setAccountName] = useAccountName();
  const { handleSubmit, register } = useForm({
    defaultValues: { name: accoutName },
    mode: "onChange",
  });

  const onSubmit = handleSubmit(({ name }) => {
    setAccountName(name);
    goBack();
  });

  return (
    <div className="text-center">
      <form
        className="dango-grid-landscape-flat-mini-l flex flex-col gap-4 text-[18px] uppercase justify-center"
        onSubmit={onSubmit}
      >
        <p className="text-typography-black-200 text-center font-extrabold tracking-[4.5px]">
          Rename Spot Account
        </p>
        <div className="flex flex-col w-full">
          <Input {...register("name")} />
          <Button type="submit">Save</Button>
        </div>
      </form>
      <Button variant="light" onClick={goBack}>
        Back
      </Button>
    </div>
  );
};
