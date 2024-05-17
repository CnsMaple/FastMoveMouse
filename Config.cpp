#include "Config.h"

Config::Config(QWidget *parent) : QWidget(parent)
{
    generateConfig();
}

bool Config::loadConfig()
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text))
    {
        return false;
    }
    QByteArray data = file.readAll();
    file.close();
    QJsonDocument doc = QJsonDocument::fromJson(data);
    if (doc.isNull())
    {
        qDebug() << "Failed to create JSON doc.";
        return false;
    }
    QJsonObject obj = doc.object();
    QString version = obj.value("version").toString();
    qDebug() << version;

    for (auto it = jsonMapQList.begin(); it != jsonMapQList.end(); it++)
    {
        QJsonArray array = obj.value(it.key()).toArray();
        it.value().clear();
        for (int i = 0; i < array.size(); i++)
        {
            it.value().append(array.at(i).toInt());
        }
    }

    for (auto it = jsonMapInt.begin(); it != jsonMapInt.end(); it++)
    {
        it.value() = obj.value(it.key()).toInt();
    }

    return true;
}

bool Config::saveConfig()
{
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text))
    {
        return false;
    }
    QJsonObject obj;
    obj.insert("version", "1.0.0");

    // 遍历QMap jsonMap
    for (auto it = jsonMapQList.begin(); it != jsonMapQList.end(); it++)
    {
        QString key = it.key();
        QList<int> value = it.value();
        QJsonArray valueArray;
        for (int i = 0; i < value.size(); i++)
        {
            valueArray.append(value.at(i));
        }
        obj.insert(key, valueArray);
    }

    for (auto it = jsonMapInt.begin(); it != jsonMapInt.end(); it++)
    {
        obj.insert(it.key(), it.value());
    }

    QJsonDocument doc(obj);
    QByteArray data = doc.toJson();
    file.write(data);
    file.close();
    return true;
}

void Config::generateConfig(bool generateNew)
{
    // 判断config.json是否存在
    QFile file(path);
    QMap<QString, QList<int>> rawJsonMap = {
        {"openJumpMouse",    { 164, 160 }     },
        { "exitJumpMouse",   { 27, 164 }      },
        { "mouseLeft",       { 78, 164 }      },
        { "mouseMiddle",     { 77, 164 }      },
        { "mouseRight",      { 188, 164 }     },
        { "mouseLongLeft",   { 78, 164, 160 } },
        { "mouseLongMiddle", { 77, 164, 160 } },
        { "mouseLongRight",  { 188, 164, 160 }},
        { "mouseMoveUp",     { 75, 164 }      },
        { "mouseMoveDown",   { 74, 164 }      },
        { "mouseMoveLeft",   { 72, 164 }      },
        { "mouseMoveRight",  { 76, 164 }      },
        { "mouseMoveAddSpeed",  { 9, 164 }      },
    };

    QMap<QString, int> rawJsonMapInt = {
        {"mouseMoveUpSize",        1 },
        { "mouseMoveDownSize",     1 },
        { "mouseMoveLeftSize",     1 },
        { "mouseMoveRightSize",    1 },
        { "mouseMoveUpMaxSize",    30},
        { "mouseMoveDownMaxSize",  30},
        { "mouseMoveLeftMaxSize",  30},
        { "mouseMoveRightMaxSize", 30},
    };

    if (jsonMapQList.size() != rawJsonMap.size())
    {
        QMessageBox::information(this, "提示", "配置文件错误");
        return;
    }
    if (generateNew)
    {
        for (auto it = jsonMapQList.begin(); it != jsonMapQList.end(); it++)
        {
            QString key = it.key();
            QList<int> value = it.value();
            jsonMapQList[key] = rawJsonMap[key];
        }

        for (auto it = jsonMapInt.begin(); it != jsonMapInt.end(); it++)
        {
            jsonMapInt[it.key()] = rawJsonMapInt[it.key()];
        }

        saveConfig();
    }
    else if (file.exists())
    {
        // 合并新设置
        loadConfig();
        for (auto it = jsonMapQList.begin(); it != jsonMapQList.end(); it++)
        {
            QString key = it.key();
            QList<int> value = it.value();
            if (value.size() == 0)
            {
                jsonMapQList[key] = rawJsonMap[key];
            }
        }

        for (auto it = jsonMapInt.begin(); it != jsonMapInt.end(); it++)
        {
            if (it.value() == 0)
            {
                jsonMapInt[it.key()] = rawJsonMapInt[it.key()];
            }
        }

        saveConfig();
        return;
    }
    else
    {
        // 生成路径
        QDir dir("res");
        dir.mkpath(".");

        for (auto it = jsonMapQList.begin(); it != jsonMapQList.end(); it++)
        {
            QString key = it.key();
            QList<int> value = it.value();
            jsonMapQList[key] = rawJsonMap[key];
        }

        for (auto it = jsonMapInt.begin(); it != jsonMapInt.end(); it++)
        {
            jsonMapInt[it.key()] = rawJsonMapInt[it.key()];
        }
        saveConfig();
    }
    file.close();
    return;
}
