import Top from './Top'
import {Layout, Model} from 'flexlayout-react';
import 'flexlayout-react/style/light.css';  
import FlexLayout from "./FlexLayout";






export default function Dashboard() {
  return (
    <div className='w-full h-[100vh] bg-slate-500'>
      <Top />
      <FlexLayout/>
      Dashboard stuff
    </div>
  )
}
