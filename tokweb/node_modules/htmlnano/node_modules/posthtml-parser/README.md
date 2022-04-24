# posthtml-parser
[![npm version](https://badge.fury.io/js/posthtml-parser.svg)](http://badge.fury.io/js/posthtml-parser)
[![Build Status](https://travis-ci.org/posthtml/posthtml-parser.svg?branch=master)](https://travis-ci.org/posthtml/posthtml-parser?branch=master)
[![Coverage Status](https://coveralls.io/repos/posthtml/posthtml-parser/badge.svg?branch=master)](https://coveralls.io/r/posthtml/posthtml-parser?branch=master)

Parse HTML/XML to [PostHTMLTree](https://github.com/posthtml/posthtml#posthtml-json-tree-example). 
More about [PostHTML](https://github.com/posthtml/posthtml#readme)

## Install

[NPM](http://npmjs.com) install
```
$ npm install posthtml-parser
```

## Usage 

#### input HTML
```html
<a class="animals" href="#">
    <span class="animals__cat" style="background: url(cat.png)">Cat</span>
</a>
```
```js
var parser = require('posthtml-parser');
var fs = require('fs');
var html = fs.readFileSync('path/to/input.html').toString();

clonsole.log(parser(html)); // Look #Result PostHTMLTree
```

#### input HTML
```html
<a class="animals" href="#">
    <span class="animals__cat" style="background: url(cat.png)">Cat</span>
</a>
```

#### Result PostHTMLTree
```js
[{
    tag: 'a',
    attrs: {
        class: 'animals',
        href: '#'
    },
    content: [
        '\n    ',
            {
            tag: 'span',
            attrs: {
                class: 'animals__cat',
                style: 'background: url(cat.png)'
            },
            content: ['Cat']
        },
        '\n'
    ]
}]
```

## License

[MIT](LICENSE)
