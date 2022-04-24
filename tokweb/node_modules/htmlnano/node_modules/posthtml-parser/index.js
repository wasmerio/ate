/*jshint -W082 */
var htmlparser = require('htmlparser2');

/**
 * Parse html to PostHTMLTree
 * @param  {String} html
 * @return {Object}
 */
module.exports = function postHTMLParser(html) {
    var bufArray = [],
        results = [];

    bufArray.last = function() {
        return this[this.length - 1];
    };

    var parser = new htmlparser.Parser({
        onprocessinginstruction: function(name, data) {
            name.toLowerCase() === '!doctype' && results.push('<' + data + '>');
        },
        oncomment: function(data) {
            var comment = '<!--' + data + '-->',
                last = bufArray.last();

            if (!last) {
                results.push(comment);
                return;
            }

            last.content || (last.content = []);
            last.content.push(comment);
        },
        onopentag: function(tag, attrs) {
            var buf = {};

            buf.tag = tag;

            if (!isEmpty(attrs)) buf.attrs = attrs;

            bufArray.push(buf);
        },
        onclosetag: function() {
            var buf = bufArray.pop();

            if (bufArray.length === 0) {
                results.push(buf);
                return;
            }

            var last = bufArray.last();
            if (!(last.content instanceof Array)) {
                last.content = [];
            }
            last.content.push(buf);
        },
        ontext: function(text) {
            var last = bufArray.last();
            if (!last) {
                results.push(text);
                return;
            }

            last.content || (last.content = []);
            last.content.push(text);
        }
    }, {lowerCaseTags: false});

    parser.write(html);
    parser.end();

    return results;
};

function isEmpty(obj) {
    for (var key in obj) {
        if (Object.prototype.hasOwnProperty.call(obj, key)) {
            return false;
        }
    }
    return true;
}
