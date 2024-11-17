import Top from './Top'
import FlexLayout from './FlexLayout'

export default function Dashboard() {
  return (
    <div className='w-full h-[100vh] bg-slate-500'>
      {/* Need to fix top nav later, but its fine rn. the flexlayout Layout component takes up full screen by default or something*/}
      {/* <Top /> */}
      <FlexLayout />
    </div>
  )
}
