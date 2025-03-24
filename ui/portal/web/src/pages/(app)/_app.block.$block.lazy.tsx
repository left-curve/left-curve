import {
  IconCopy,
  Table,
  type TableColumn,
  TruncateText,
  useMediaQuery,
} from "@left-curve/applets-kit";
import type { IndexedTransaction } from "@left-curve/dango/types";
import { usePublicClient } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";
import { createLazyFileRoute } from "@tanstack/react-router";
import { BlockPageSkeleton } from "~/components/skeletons/block-page";

export const Route = createLazyFileRoute("/(app)/_app/block/$block")({
  component: RouteComponent,
});

function RouteComponent() {
  const { block } = Route.useParams();
  const client = usePublicClient();
  const { isMd } = useMediaQuery();

  const { data: blockDetails, isLoading } = useQuery({
    queryKey: ["block", block],
    queryFn: async () => {
      const height = +block;
      const blockInfo = await client.queryBlock({ height });
      return {
        ...blockInfo,
        proposer: "Leftcurve Validator",
      };
    },
  });

  if (isLoading) return <BlockPageSkeleton />;

  if (!blockDetails) {
    return <div>Not found</div>;
  }

  const { proposer, transactions, createdAt, blockHeight, hash } = blockDetails;

  const columns: TableColumn<IndexedTransaction> = [
    {
      header: "Type",
      cell: ({ row }) => <p>{row.original.transactionType}</p>,
    },
    {
      header: "Hash",
      cell: ({ row }) => <TruncateText text={row.original.hash} />,
    },
    {
      header: "Account",
      cell: ({ row }) => <p>{row.original.sender}</p>,
    },
    {
      header: "Result",
      cell: ({ row }) => {
        const { hasSucceeded } = row.original;
        return (
          <p className={hasSucceeded ? "text-status-success" : "text-status-fail"}>
            {hasSucceeded ? "Success" : "Fail"}
          </p>
        );
      },
    },
  ];

  return (
    <div className="w-full md:max-w-[76rem] flex flex-col gap-6 p-4 pt-6 mb-16">
      <div className="flex flex-col rounded-md px-4 py-3 bg-rice-25 shadow-card-shadow text-gray-700 diatype-m-bold relative overflow-hidden">
        <div className="overflow-y-auto scrollbar-none w-full gap-4 flex flex-col">
          <h1 className="h4-bold">Block Detail</h1>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
            <div className="col-span-1 md:col-span-2 flex items-center gap-1">
              <p className="diatype-md-medium text-gray-500">Block Hash:</p>
              {isMd ? <p>{hash}</p> : <TruncateText text={hash} />}
              <IconCopy className="w-4 h-4 cursor-pointer" copyText={hash} />
            </div>
            <div className="flex items-center gap-1">
              <p className="diatype-md-medium text-gray-500">Block Height:</p>
              <p>{blockHeight}</p>
            </div>
            <div className="flex items-center gap-1">
              <p className="diatype-md-medium text-gray-500">Proposer:</p>
              <p>{proposer}</p>
            </div>
            <div className="flex items-center gap-1">
              <p className="diatype-md-medium text-gray-500">Number of Tx:</p>
              <p>{transactions.length}</p>
            </div>
            <div className="flex items-center gap-1">
              <p className="diatype-md-medium text-gray-500">Time:</p>
              <p>{new Date(createdAt).toISOString()}</p>
            </div>
          </div>
          {isMd ? (
            <img
              src="/images/emojis/detailed/map-explorer.svg"
              alt="map-emoji"
              className="w-[16.25rem] h-[16.25rem] opacity-40 absolute top-[-2rem] right-[2rem] mix-blend-multiply"
            />
          ) : null}
        </div>
      </div>

      {transactions.length ? <Table data={transactions} columns={columns} /> : null}
    </div>
  );
}
