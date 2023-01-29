import { Lobster } from '../pkg';

(async () => {
  const a = new Lobster();
  console.log(a.exec({ type: 'Limit', id: 123123132121111n, side: 'Ask', qty: 3, price: 120 }));
  
})()