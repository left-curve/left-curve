import { Select } from "@left-curve/applets-kit";
import { SelectProps } from "../../../../../applets/kit/src/components/Select";

interface Props extends Omit<SelectProps, "children"> {
  networks: { name: string; id: string; time: string }[];
  selectedNetwork?: { name: string; id: string; time: string };
  onNetworkChange: (network: { name: string; id: string; time: string }) => void;
}

export const NetworkSelector: React.FC<Props> = ({
  selectedNetwork,
  onNetworkChange,
  networks,
  classNames,
  ...props
}) => {
  const handleNetworkChange = (id: string) => {
    const network = networks.find((n) => n.id === id);
    if (network) {
      onNetworkChange(network);
    }
  };

  return (
    <Select
      {...props}
      onChange={handleNetworkChange}
      label="Select Network"
      placeholder="Select Network"
      classNames={{
        base: `w-full`,
        listboxWrapper: `${classNames?.listboxWrapper}`,
        listbox: `${classNames?.listbox}`,
        value: `${classNames?.value}`,
        trigger: `${classNames?.trigger}`,
      }}
    >
      {networks.map((network) => (
        <Select.Item key={network.id} value={network.id}>
          <div className="flex flex-col text-ink-primary-900 items-start">
            <p className="diatype-m-medium">{network.name}</p>
            <p className="diatype-sm-regular text-ink-tertiary-500">{network.time}</p>
          </div>
        </Select.Item>
      ))}
    </Select>
  );
};
