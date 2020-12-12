import * as wasm from "shoppinglist";

let shopping_list = new wasm.ShoppingListState();

function processShoppingList() {
    const input_str = document.getElementById("slinput");
    shopping_list.process_input(input_str.value);

    const sorted = shopping_list.get_plaintext_result();

    const blob = new Blob([sorted], {"type":"text/plain"});
    const blob_url = URL.createObjectURL(blob);

    let url_div = document.getElementById("contentastext");
    if(url_div.firstChild) {
        url_div.removeChild(url_div.firstChild);
    }

    let new_url = document.createElement("a");
    new_url.download = "shoppinglist.txt";
    new_url.href = blob_url;
    new_url.appendChild(document.createTextNode("Download shopping list"));
    url_div.appendChild(new_url);

    const list_result = shopping_list.get_list_result();
    let shopping_list_ul = document.getElementById("shoppinglist");
    let shopping_list_div = shopping_list_ul.parentNode;
    shopping_list_div.removeChild(shopping_list_ul);
    let shopping_list_ul_new = document.createElement("ul");
    shopping_list_ul_new.id = "shoppinglist";
    shopping_list_ul_new.className = "list-group";
    for (let merchant of list_result) {
        let merchant_li = document.createElement("li");
        merchant_li.className = "list-group-item";
        merchant_li.appendChild(document.createTextNode(merchant.name));
        let items_ul = document.createElement("ul");
        items_ul.className = "shoppinglist-items";
        for (let merch_item of merchant.items) {
            let item_li = document.createElement("li");
            item_li.appendChild(document.createTextNode(merch_item));
            items_ul.appendChild(item_li);
        }
        merchant_li.appendChild(items_ul);
        shopping_list_ul_new.appendChild(merchant_li);
    }
    shopping_list_div.appendChild(shopping_list_ul_new);
}

function loadConfigFile() {
    const configfile = this.files[0];
    configfile.text().then(config_str => shopping_list.load_config(config_str));
}

document.getElementById("shopbtn").onclick = processShoppingList;
document.getElementById("configfile").addEventListener("change", loadConfigFile, false);
