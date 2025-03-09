import { createFileRoute } from '@tanstack/react-router'

import { AccordionItem, IconCopy } from '@left-curve/applets-kit'
import { useQuery } from '@tanstack/react-query'

export const Route = createFileRoute('/(app)/_app/tx/$txHash')({
  component: RouteComponent,
})

function RouteComponent() {
  const { txHash } = Route.useParams()

  const { data: txDetails } = useQuery({
    queryKey: ['tx', txHash],
    queryFn: () => {
      return {
        sender: 'Faucet',
        index: 1,
        block: 123456,
        time: new Date(),
        events: [
          {
            type: 'Transfer',
            details: {},
          },
        ],
        message: {},
      }
    },
  })

  if (!txDetails) {
    return <div>Not found</div>
  }

  const { sender, index, block, time, events, message } = txDetails

  return (
    <div className="w-full md:max-w-[76rem] flex flex-col gap-6 p-4 pt-6 mb-16">
      <div className="w-full shadow-card-shadow bg-rice-50 rounded-3xl p-4">
        <div className="flex flex-col gap-4 rounded-md px-4 py-3 bg-rice-25 shadow-card-shadow text-gray-700 diatype-m-bold relative overflow-hidden">
          <h1 className="h4-bold">Transaction Detail</h1>
          <div className="flex gap-1 items-center">
            <p className="diatype-md-medium text-gray-500">Tx hash:</p>
            <p>{txHash}</p>
            <IconCopy className="w-4 h-4 text-gray-500" copyText={txHash} />
          </div>
          <div className="grid grid-cols-1 md:grid-cols-2">
            <div className="flex items-center gap-1">
              <p className="diatype-md-medium text-gray-500">Sender:</p>
              <p>{sender}</p>
            </div>
            <div className="flex items-center gap-1">
              <p className="diatype-md-medium text-gray-500">Time:</p>
              <p>{time.toISOString()}</p>
            </div>
            <div className="flex items-center gap-1">
              <p className="diatype-md-medium text-gray-500">Block:</p>
              <p>{block}</p>
            </div>
            <div className="flex items-center gap-1">
              <p className="diatype-md-medium text-gray-500">Index:</p>
              <p>{index}</p>
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
        <p className="h4-bold">Message</p>
        <AccordionItem text="Message">
          <div className="p-4 bg-gray-700 shadow-card-shadow  rounded-md text-white-100">
            {JSON.stringify(message)}
          </div>
        </AccordionItem>
        {events.length ? <p className="h4-bold">Events</p> : null}
        {events.map((event) => (
          <AccordionItem key={crypto.randomUUID()} text={event.type}>
            <div className="p-4 bg-gray-700 shadow-card-shadow  rounded-md text-white-100">
              {JSON.stringify(event.details)}
            </div>
          </AccordionItem>
        ))}
      </div>
    </div>
  )
}
