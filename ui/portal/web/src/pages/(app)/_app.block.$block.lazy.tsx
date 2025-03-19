import { Table } from '@left-curve/applets-kit'
import { useQuery } from '@tanstack/react-query'
import { createLazyFileRoute } from '@tanstack/react-router'

const mockData = {
  block: {
    height: 123456,
    timestamp: new Date(),
    hash: '5AE2D3C26F327C9AB4A5EB1151DF3358988C5AC5899252EC9251271A20CF0148',
  },
  proposer: '',
  txCount: 0,
}

export const Route = createLazyFileRoute('/(app)/_app/block/$block')({
  component: RouteComponent,
})

function RouteComponent() {
  const { block } = Route.useParams()

  const { data: blockDetails } = useQuery({
    queryKey: ['block', block],
    queryFn: () => {
      return {
        proposer: 'Faucet',
        height: 123456,
        timestamp: new Date(),
        txs: [],
      }
    },
  })

  if (!blockDetails) {
    return <div>Not found</div>
  }

  const { proposer, txs, timestamp, height } = blockDetails

  return (
    <div className="w-full md:max-w-[76rem] flex flex-col gap-6 p-4 pt-6 mb-16">
      <div className="w-full shadow-card-shadow bg-rice-50 rounded-3xl p-4">
        <div className="flex flex-col gap-4 rounded-md px-4 py-3 bg-rice-25 shadow-card-shadow text-gray-700 diatype-m-bold relative overflow-hidden">
          <h1 className="h4-bold">Block Detail</h1>
          <div className="grid grid-cols-1 md:grid-cols-2">
            <div className="flex items-center gap-1">
              <p className="diatype-md-medium text-gray-500">Block Height:</p>
              <p>{height}</p>
            </div>
            <div className="flex items-center gap-1">
              <p className="diatype-md-medium text-gray-500">Proposer:</p>
              <p>{proposer}</p>
            </div>
            <div className="flex items-center gap-1">
              <p className="diatype-md-medium text-gray-500">Number of Tx:</p>
              <p>{txs.length}</p>
            </div>
            <div className="flex items-center gap-1">
              <p className="diatype-md-medium text-gray-500">Time:</p>
              <p>{timestamp.toISOString()}</p>
            </div>
          </div>
          <img
            src="/images/emojis/map-no-simple.svg"
            alt="map-emoji"
            className="w-[16.25rem] h-[16.25rem] opacity-40 absolute top-[-2rem] right-[2rem] mix-blend-multiply"
          />
        </div>
      </div>
      <div className="w-full shadow-card-shadow bg-rice-25 rounded-3xl p-4 flex flex-col gap-4">
        {/* <Table /> */}
      </div>
    </div>
  )
}
