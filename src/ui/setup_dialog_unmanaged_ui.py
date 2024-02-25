# -*- coding: utf-8 -*-

################################################################################
## Form generated from reading UI file 'setup-dialog-unmanaged.ui'
##
## Created by: Qt User Interface Compiler version 6.6.2
##
## WARNING! All changes made in this file will be lost when recompiling UI file!
################################################################################

try:
    from PySide6.QtCore import (QCoreApplication, QDate, QDateTime, QLocale,
        QMetaObject, QObject, QPoint, QRect,
        QSize, QTime, QUrl, Qt)
    from PySide6.QtGui import (QBrush, QColor, QConicalGradient, QCursor,
        QFont, QFontDatabase, QGradient, QIcon,
        QImage, QKeySequence, QLinearGradient, QPainter,
        QPalette, QPixmap, QRadialGradient, QTransform)
    from PySide6.QtWidgets import (QApplication, QComboBox, QDialog, QFormLayout,
        QLabel, QLayout, QLineEdit, QPushButton,
        QSizePolicy, QWidget)
except ImportError:
    from PyQt5.QtCore import (QCoreApplication, QDate, QDateTime, QLocale,
        QMetaObject, QObject, QPoint, QRect,
        QSize, QTime, QUrl, Qt)
    from PyQt5.QtGui import (QBrush, QColor, QConicalGradient, QCursor,
        QFont, QFontDatabase, QGradient, QIcon,
        QImage, QKeySequence, QLinearGradient, QPainter,
        QPalette, QPixmap, QRadialGradient, QTransform)
    from PyQt5.QtWidgets import (QApplication, QComboBox, QDialog, QFormLayout,
        QLabel, QLayout, QLineEdit, QPushButton,
        QSizePolicy, QWidget)

from . import uds_rc

class Ui_UdsActorSetupDialog(object):
    def setupUi(self, UdsActorSetupDialog):
        if not UdsActorSetupDialog.objectName():
            UdsActorSetupDialog.setObjectName(u"UdsActorSetupDialog")
        UdsActorSetupDialog.setWindowModality(Qt.WindowModal)
        UdsActorSetupDialog.resize(601, 243)
        sizePolicy = QSizePolicy(QSizePolicy.Policy.Preferred, QSizePolicy.Policy.Preferred)
        sizePolicy.setHorizontalStretch(0)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(UdsActorSetupDialog.sizePolicy().hasHeightForWidth())
        UdsActorSetupDialog.setSizePolicy(sizePolicy)
        font = QFont()
        font.setFamilies([u"Verdana"])
        font.setPointSize(9)
        UdsActorSetupDialog.setFont(font)
        UdsActorSetupDialog.setContextMenuPolicy(Qt.DefaultContextMenu)
        icon = QIcon()
        icon.addFile(u":/img/img/uds-icon.png", QSize(), QIcon.Normal, QIcon.Off)
        UdsActorSetupDialog.setWindowIcon(icon)
        UdsActorSetupDialog.setAutoFillBackground(False)
        UdsActorSetupDialog.setLocale(QLocale(QLocale.English, QLocale.UnitedStates))
        UdsActorSetupDialog.setSizeGripEnabled(False)
        UdsActorSetupDialog.setModal(True)
        self.saveButton = QPushButton(UdsActorSetupDialog)
        self.saveButton.setObjectName(u"saveButton")
        self.saveButton.setEnabled(True)
        self.saveButton.setGeometry(QRect(10, 210, 181, 23))
        self.saveButton.setMinimumSize(QSize(181, 0))
        self.saveButton.setContextMenuPolicy(Qt.DefaultContextMenu)
        self.closeButton = QPushButton(UdsActorSetupDialog)
        self.closeButton.setObjectName(u"closeButton")
        self.closeButton.setGeometry(QRect(410, 210, 171, 23))
        sizePolicy1 = QSizePolicy(QSizePolicy.Policy.Preferred, QSizePolicy.Policy.Fixed)
        sizePolicy1.setHorizontalStretch(0)
        sizePolicy1.setVerticalStretch(0)
        sizePolicy1.setHeightForWidth(self.closeButton.sizePolicy().hasHeightForWidth())
        self.closeButton.setSizePolicy(sizePolicy1)
        self.closeButton.setMinimumSize(QSize(171, 0))
        self.testButton = QPushButton(UdsActorSetupDialog)
        self.testButton.setObjectName(u"testButton")
        self.testButton.setEnabled(False)
        self.testButton.setGeometry(QRect(210, 210, 181, 23))
        self.testButton.setMinimumSize(QSize(181, 0))
        self.layoutWidget = QWidget(UdsActorSetupDialog)
        self.layoutWidget.setObjectName(u"layoutWidget")
        self.layoutWidget.setGeometry(QRect(10, 10, 571, 191))
        self.formLayout = QFormLayout(self.layoutWidget)
        self.formLayout.setObjectName(u"formLayout")
        self.formLayout.setSizeConstraint(QLayout.SetDefaultConstraint)
        self.formLayout.setFieldGrowthPolicy(QFormLayout.AllNonFixedFieldsGrow)
        self.formLayout.setVerticalSpacing(16)
        self.formLayout.setContentsMargins(0, 0, 0, 0)
        self.label_security = QLabel(self.layoutWidget)
        self.label_security.setObjectName(u"label_security")

        self.formLayout.setWidget(0, QFormLayout.LabelRole, self.label_security)

        self.validateCertificate = QComboBox(self.layoutWidget)
        self.validateCertificate.addItem("")
        self.validateCertificate.addItem("")
        self.validateCertificate.setObjectName(u"validateCertificate")

        self.formLayout.setWidget(0, QFormLayout.FieldRole, self.validateCertificate)

        self.label_host = QLabel(self.layoutWidget)
        self.label_host.setObjectName(u"label_host")

        self.formLayout.setWidget(1, QFormLayout.LabelRole, self.label_host)

        self.host = QLineEdit(self.layoutWidget)
        self.host.setObjectName(u"host")
        self.host.setAcceptDrops(False)

        self.formLayout.setWidget(1, QFormLayout.FieldRole, self.host)

        self.label_serviceToken = QLabel(self.layoutWidget)
        self.label_serviceToken.setObjectName(u"label_serviceToken")

        self.formLayout.setWidget(2, QFormLayout.LabelRole, self.label_serviceToken)

        self.serviceToken = QLineEdit(self.layoutWidget)
        self.serviceToken.setObjectName(u"serviceToken")

        self.formLayout.setWidget(2, QFormLayout.FieldRole, self.serviceToken)

        self.label_loglevel = QLabel(self.layoutWidget)
        self.label_loglevel.setObjectName(u"label_loglevel")

        self.formLayout.setWidget(4, QFormLayout.LabelRole, self.label_loglevel)

        self.logLevelComboBox = QComboBox(self.layoutWidget)
        self.logLevelComboBox.addItem(u"DEBUG")
        self.logLevelComboBox.addItem(u"INFO")
        self.logLevelComboBox.addItem(u"ERROR")
        self.logLevelComboBox.addItem(u"FATAL")
        self.logLevelComboBox.setObjectName(u"logLevelComboBox")
        self.logLevelComboBox.setFrame(True)

        self.formLayout.setWidget(4, QFormLayout.FieldRole, self.logLevelComboBox)

        self.label_restrictNet = QLabel(self.layoutWidget)
        self.label_restrictNet.setObjectName(u"label_restrictNet")

        self.formLayout.setWidget(3, QFormLayout.LabelRole, self.label_restrictNet)

        self.restrictNet = QLineEdit(self.layoutWidget)
        self.restrictNet.setObjectName(u"restrictNet")

        self.formLayout.setWidget(3, QFormLayout.FieldRole, self.restrictNet)

        self.label_host.raise_()
        self.host.raise_()
        self.label_serviceToken.raise_()
        self.serviceToken.raise_()
        self.validateCertificate.raise_()
        self.label_security.raise_()
        self.label_loglevel.raise_()
        self.logLevelComboBox.raise_()
        self.label_restrictNet.raise_()
        self.restrictNet.raise_()

        self.retranslateUi(UdsActorSetupDialog)
        self.closeButton.clicked.connect(UdsActorSetupDialog.finish)
        self.testButton.clicked.connect(UdsActorSetupDialog.testUDSServer)
        self.saveButton.clicked.connect(UdsActorSetupDialog.saveConfig)
        self.host.textChanged.connect(UdsActorSetupDialog.configChanged)
        self.serviceToken.textChanged.connect(UdsActorSetupDialog.configChanged)
        self.restrictNet.textChanged.connect(UdsActorSetupDialog.configChanged)

        self.logLevelComboBox.setCurrentIndex(1)


        QMetaObject.connectSlotsByName(UdsActorSetupDialog)
    # setupUi

    def retranslateUi(self, UdsActorSetupDialog):
        UdsActorSetupDialog.setWindowTitle(QCoreApplication.translate("UdsActorSetupDialog", u"UDS Actor Configuration Tool", None))
#if QT_CONFIG(tooltip)
        self.saveButton.setToolTip(QCoreApplication.translate("UdsActorSetupDialog", u"Click to register Actor with UDS Broker", None))
#endif // QT_CONFIG(tooltip)
#if QT_CONFIG(whatsthis)
        self.saveButton.setWhatsThis(QCoreApplication.translate("UdsActorSetupDialog", u"<html><head/><body><p>Click on this button to register Actor with UDS Broker.</p></body></html>", None))
#endif // QT_CONFIG(whatsthis)
        self.saveButton.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Save Configuration", None))
#if QT_CONFIG(tooltip)
        self.closeButton.setToolTip(QCoreApplication.translate("UdsActorSetupDialog", u"Closes UDS Actor Configuration (discard pending changes if any)", None))
#endif // QT_CONFIG(tooltip)
#if QT_CONFIG(whatsthis)
        self.closeButton.setWhatsThis(QCoreApplication.translate("UdsActorSetupDialog", u"<html><head/><body><p>Exits the UDS Actor Configuration Tool</p></body></html>", None))
#endif // QT_CONFIG(whatsthis)
        self.closeButton.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Close", None))
#if QT_CONFIG(tooltip)
        self.testButton.setToolTip(QCoreApplication.translate("UdsActorSetupDialog", u"Click to test existing configuration (disabled if no config found)", None))
#endif // QT_CONFIG(tooltip)
#if QT_CONFIG(whatsthis)
        self.testButton.setWhatsThis(QCoreApplication.translate("UdsActorSetupDialog", u"<html><head/><body><p>Click on this button to test the server host and assigned toen.</p></body></html>", None))
#endif // QT_CONFIG(whatsthis)
        self.testButton.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Test configuration", None))
        self.label_security.setText(QCoreApplication.translate("UdsActorSetupDialog", u"SSL Validation", None))
        self.validateCertificate.setItemText(0, QCoreApplication.translate("UdsActorSetupDialog", u"Ignore certificate", None))
        self.validateCertificate.setItemText(1, QCoreApplication.translate("UdsActorSetupDialog", u"Verify certificate", None))

#if QT_CONFIG(tooltip)
        self.validateCertificate.setToolTip(QCoreApplication.translate("UdsActorSetupDialog", u"Select communication security with broker", None))
#endif // QT_CONFIG(tooltip)
#if QT_CONFIG(whatsthis)
        self.validateCertificate.setWhatsThis(QCoreApplication.translate("UdsActorSetupDialog", u"<html><head/><body><p>Select the security for communications with UDS Broker.</p><p>The recommended method of communication is <span style=\" font-weight:600;\">Use SSL</span>, but selection needs to be acording to your broker configuration.</p></body></html>", None))
#endif // QT_CONFIG(whatsthis)
        self.label_host.setText(QCoreApplication.translate("UdsActorSetupDialog", u"UDS Server", None))
#if QT_CONFIG(tooltip)
        self.host.setToolTip(QCoreApplication.translate("UdsActorSetupDialog", u"Uds Broker Server Addres. Use IP or FQDN", None))
#endif // QT_CONFIG(tooltip)
#if QT_CONFIG(whatsthis)
        self.host.setWhatsThis(QCoreApplication.translate("UdsActorSetupDialog", u"Enter here the UDS Broker Addres using either its IP address or its FQDN address", None))
#endif // QT_CONFIG(whatsthis)
        self.label_serviceToken.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Service Token", None))
#if QT_CONFIG(tooltip)
        self.serviceToken.setToolTip(QCoreApplication.translate("UdsActorSetupDialog", u"UDS Service Token", None))
#endif // QT_CONFIG(tooltip)
#if QT_CONFIG(whatsthis)
        self.serviceToken.setWhatsThis(QCoreApplication.translate("UdsActorSetupDialog", u"<html><head/><body><p>Token of the service on UDS platform</p><p>This token can be obtainend from the service configuration on UDS.</p></body></html>", None))
#endif // QT_CONFIG(whatsthis)
        self.label_loglevel.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Log Level", None))

        self.label_restrictNet.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Restrict Net", None))
#if QT_CONFIG(tooltip)
        self.restrictNet.setToolTip(QCoreApplication.translate("UdsActorSetupDialog", u"Restrict valid detection of network interfaces to this network.", None))
#endif // QT_CONFIG(tooltip)
#if QT_CONFIG(whatsthis)
        self.restrictNet.setWhatsThis(QCoreApplication.translate("UdsActorSetupDialog", u"<html><head/><body><p>Restrics valid detection of network interfaces.</p><p>Note: Use this field only in case of several network interfaces, so UDS knows which one is the interface where the user will be connected..</p></body></html>", None))
#endif // QT_CONFIG(whatsthis)
    # retranslateUi

