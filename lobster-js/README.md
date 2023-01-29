# lobster-js

Nodejs binding for [lobster](https://docs.rs/lobster/latest/lobster/)

## Usage

```javascript

import { Loobster } from 'lobster-js';


const book = new Loobster();

let result = book.execute({
  type: 'Limit',
  id: 112,
  side: "Bid",
  qty: 100,
  price: 100
});




```