import type React from "react";

interface TableProps {
  bottomContent?: React.ReactNode;
  topContent?: React.ReactNode;
}

export const Table: React.FC<TableProps> = ({ topContent, bottomContent }) => {
  return (
    <div className="bg-rice-25 shadow-card-shadow flex flex-col rounded-3xl w-full p-4 gap-4">
      {topContent}
      <div className="overflow-y-auto scrollbar-none w-full">
        <table className="table-auto w-full">
          {/* Header */}
          <thead>
            <tr className=" text-[#717680] font-semibold text-xs">
              <th className="text-start rounded-l-xl p-4 bg-green-bean-100">Vault</th>
              <th className="text-end bg-green-bean-100 p-4">Type</th>
              <th className="text-end bg-green-bean-100 p-4">APR</th>
              <th className="text-end bg-green-bean-100 p-4">
                <p className="min-w-fit">Liquidity Available</p>
              </th>
              <th className="text-end bg-green-bean-100 p-4">TVL</th>
              <th className="text-end rounded-r-xl bg-green-bean-100 p-4">Risk Level</th>
            </tr>
          </thead>
          {/* <div className="grid grid-cols-6 gap-4 p-4 rounded-md bg-green-bean-100 text-[#717680] font-semibold">
              <p className="text-xs text-gray-500">Vault</p>
              <p className="text-xs text-gray-500 flex items-center justify-end">Type</p>
              <p className="text-xs text-gray-500 flex items-center justify-end">APR</p>
              <p className="text-xs text-gray-500 flex items-center justify-end">
                Liquidity Available
              </p>
              <p className="text-xs text-gray-500 flex items-center justify-end">TVL</p>
              <p className="text-xs text-gray-500 text-end">Risk Level</p>
            </div> */}

          {/* rows */}
          <tbody>
            {Array.from([1, 2, 3, 4, 5]).map((e) => {
              return (
                <tr className="p-4 border-b border-b-gray-100" key={`row-${e}`}>
                  <td className="p-4">
                    <div className="flex gap-2 text-lg">
                      <div className="flex">
                        <img
                          src="https://w7.pngwing.com/pngs/268/1013/png-transparent-ethereum-eth-hd-logo-thumbnail.png"
                          alt=""
                          className="h-6 min-w-6 rounded-full"
                        />
                        <img
                          src="https://w7.pngwing.com/pngs/268/1013/png-transparent-ethereum-eth-hd-logo-thumbnail.png"
                          alt=""
                          className="h-6 min-w-6 -ml-1 rounded-full"
                        />
                      </div>
                      <p className="min-w-fit">ETH-USD</p>
                    </div>
                  </td>
                  <td className="p-4">
                    <div className="flex items-center justify-end">
                      <div className="text-xs bg-green-bean-200 border border-green-bean-300 text-green-bean-700 py-1 px-2 rounded-[4px] h-fit w-fit">
                        Lending
                      </div>
                    </div>
                  </td>
                  <td className="p-4">
                    <div className="flex items-center justify-end">17.72%</div>
                  </td>
                  <td className="p-4">
                    <div className="flex items-center justify-end">15.63%</div>
                  </td>
                  <td className="p-4">
                    <div className="flex items-center justify-end">15.63%</div>
                  </td>
                  <td className="p-4">
                    <div className="flex items-center justify-end">Low</div>
                  </td>
                </tr>
                /*  <div
                    className="grid grid-cols-6 gap-4 p-4 border-b border-b-gray-100"
                    key={`row-${e}`}
                  >
                    <div className="flex">
                      <div className="flex gap-2 text-lg">
                        <div className="flex">
                          <img
                            src="https://w7.pngwing.com/pngs/268/1013/png-transparent-ethereum-eth-hd-logo-thumbnail.png"
                            alt=""
                            className="h-6 w-6 rounded-full"
                          />
                          <img
                            src="https://w7.pngwing.com/pngs/268/1013/png-transparent-ethereum-eth-hd-logo-thumbnail.png"
                            alt=""
                            className="h-6 w-6 -ml-1 rounded-full"
                          />
                        </div>
                        <p>ETH-USD</p>
                      </div>
                    </div>
                    <div className="flex items-center justify-end">
                      <div className="text-xs bg-green-bean-200 border border-green-bean-300 text-green-bean-700 py-1 px-2 rounded-[4px] h-fit w-fit">
                        Lending
                      </div>
                    </div>
                    <div className="flex items-center justify-end">17.72%</div>
                    <div className="flex items-center justify-end">15.63%</div>
                    <div className="flex items-center justify-end">15.63%</div>
                    <div className="flex items-center justify-end">Lo w</div>
                  </div> */
              );
            })}
          </tbody>
        </table>
      </div>
      {bottomContent}
    </div>
  );
};
