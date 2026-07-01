import { Select, type SelectProps } from "./Select";

export type NetworkSelectorNetwork = {
  name: string;
  id: string;
  time: string;
  withdrawLiquidity?: string;
};

export interface NetworkSelectorProps extends Omit<SelectProps, "children"> {
  networks: NetworkSelectorNetwork[];
  selectedNetwork?: NetworkSelectorNetwork;
  onNetworkChange: (network: NetworkSelectorNetwork) => void;
}

export const NetworkSelector: React.FC<NetworkSelectorProps> = ({
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
            <p className="diatype-sm-regular text-ink-tertiary-500">
              <span>{network.time}</span>
              {network.withdrawLiquidity ? (
                <>
                  <span className="px-1">·</span>
                  <span>{network.withdrawLiquidity}</span>
                </>
              ) : null}
            </p>
          </div>
        </Select.Item>
      ))}
    </Select>
  );
};
